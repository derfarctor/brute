#![allow(dead_code)]
#![allow(unused_imports)]
use std::thread;
use std::time::Instant;
use std::env;
use std::process;
use std::sync::{Arc, Mutex};
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;
use tokio::time::{sleep, Duration};
use itertools::Itertools;
use serde_json::{Value, json};
use toml;
use colour::{e_cyan, e_green};

mod utils;
use crate::utils::mnemonic;

type Terminator = Arc<Mutex<bool>>;
type KeysTested = Arc<AtomicUsize>;

const BATCH_SIZE: usize = 4;
const NODE_URL: &str = "https://api.nanos.cc/";
//const NODE_URL: &str = "https://kaliumapi.appditto.com/api";

struct Account {
        address: String,
        seed_bytes: [u8; 32],
}

#[tokio::main]
async fn main() {
        let args: Vec<String> = env::args().collect();
        
        let (broken_mnemonic, stop_at_first) = handle_args(&args);

        if !mnemonic::is_valid_broken_mnemonic(&broken_mnemonic) {
                    process::exit(0);
        }

        brute(broken_mnemonic, stop_at_first).await;
}

async fn brute(broken_mnemonic: [&str; 24], stop_at_first: bool) {

        let (possibilities, unknown_indexes, complexity) = mnemonic::get_info(&broken_mnemonic);

        if complexity == 0 {
                eprintln!("The mnemonic has too many unknown parts to be calculated (Complexity greater than uint64 limit). Remove a complete unknown (X) or be more specific with sub-word options.");
                process::exit(0);
        }
        println!("Total mnemonic combinations: {}", complexity);

        let test_generator = possibilities.iter().map(|x| x.iter()).multi_cartesian_product();
        
        let mut test_mnemonic: [u16; 24] = mnemonic::get_test_mnemonic(&broken_mnemonic);
        let mut processes = vec![];
        let mut batch_accounts: Vec<Account> = vec![];
        let terminator = Arc::new(Mutex::new(false));

        let attempts_base = Arc::new(AtomicUsize::new(0));
        
        let log_attempts = attempts_base.clone();
        
        let logger = thread::spawn(move || {
                threaded_logger(log_attempts, complexity);
        });
        
        /*
        let logger = tokio::spawn(async move {
                async_logger(log_attempts, complexity).await;
        });
        */

        let mut last = Instant::now();
        for comb in test_generator {
                attempts_base.fetch_add(1, atomic::Ordering::Relaxed);
                for i in 0..unknown_indexes.len() {
                        test_mnemonic[unknown_indexes[i]] = *comb[i];
                }
                let (seed_bytes, valid) = mnemonic::validate_mnemonic(&test_mnemonic);
                if valid {
                        let priv_key_bytes: [u8; 32] = mnemonic::get_private_key(&seed_bytes);
                        let pub_key_bytes: [u8; 32] = mnemonic::get_public_key(&priv_key_bytes);
                        let address: String = mnemonic::get_address(&pub_key_bytes);
                        let account: Account = Account { address, seed_bytes };
                        batch_accounts.push(account);
                        if batch_accounts.len() == BATCH_SIZE {
                                if *terminator.lock().unwrap() {
                                        println!("A spawned async process found account, finishing quietly...");
                                        process::exit(0);
                                } else {
                                        let terminator = terminator.clone();
                                        let new_process = tokio::spawn(async move {
                                                return process(batch_accounts, &stop_at_first, terminator).await;
                                            });
                                        processes.push(new_process);
                                        batch_accounts = vec![];
                                }
                        }
                }
        }
        if batch_accounts.len() > 0 {
                if *terminator.lock().unwrap() {
                        println!("A spawned async process found account, finishing quietly...");
                        process::exit(0);
                } else {
                        let terminator = terminator.clone();
                        let new_process = tokio::spawn(async move {
                                return process(batch_accounts, &stop_at_first, terminator).await;
                            });
                        processes.push(new_process);
                }
        }
        let mut found_one: bool = false;

        for process in processes {
                let final_check = process.await.unwrap();
                if final_check {
                        found_one = true;
                }
        }
        if found_one {
                println!("Found opened account(s). Check brutelog.txt for more info.");
        } else {
                println!("Did not find any opened account(s).");
        }
        let mut end = terminator.lock().unwrap();
        *end = true;
}

fn threaded_logger(log_attempts: KeysTested, complexity: u64) {
        let mut last = 0;
        let start = Instant::now();
        loop {
                thread::sleep_ms(1000);
                let attempts = log_attempts.load(atomic::Ordering::Relaxed);
                /*
                e_green!("\r{} ", attempts);
                eprint!("of ");
                e_green!("{} ", complexity);
                eprint!("mnemonics tested (");
                e_green!("{:.2}%", 100.*(attempts as f64/complexity as f64));
                eprint!(") Rate: ");
                e_green!("{} ", attempts-last);
                eprint!("per second");
                */
                e_green!("\r{:.2}% Complete | {} mnemonics per second", 100.*(attempts as f64/complexity as f64), attempts as f64 / (start.elapsed().as_millis() as f64 / 1000f64));
                last = attempts;
        }
}

async fn async_logger(log_attempts: KeysTested, complexity: u64) {
        let mut last = 0;
        let start = Instant::now();
        loop {
                sleep(Duration::from_millis(1000)).await;
                let attempts = log_attempts.load(atomic::Ordering::Relaxed);
                e_green!("\r{:.2}% Complete | {} mnemonics per second", 100.*(attempts as f64/complexity as f64), attempts as f64 / (start.elapsed().as_millis() as f64 / 1000f64));
                last = attempts;
        }
}

fn handle_args(args: &[String]) -> ([&str; 24], bool) {
        let mnemonic_slice = &args[1..25];
        let mut user_mnemonic: [&str; 24] = Default::default();
        for i in 0..24 {
                user_mnemonic[i] = mnemonic_slice[i].as_str();
        }

        (user_mnemonic, true)
}

async fn process(batch: Vec<Account>, stop_at_first: &bool, terminator: Terminator) -> bool {
        let mut address_batch: Vec<String> = vec![];
        for account in batch {
                address_batch.push(account.address.clone());
        }
        let (found_account, account_addresses) = get_opened(address_batch, stop_at_first).await;
        if found_account {
                
                println!("Found opened accounts. DATA: {:?}", account_addresses);
                if *stop_at_first {
                        let mut update_found = terminator.lock().unwrap();
                        *update_found = true;
                }
                return true;
        }
        false
}

async fn get_opened(address_batch: Vec<String>, stop_at_first: &bool) -> (bool, Vec<String>) {
        let body_json = json!({
                "action":"accounts_balances",
                "accounts": address_batch
            });

        let body = body_json.to_string();
        println!("{}", body);
        let client = reqwest::Client::new();
        let res = client.post(NODE_URL)
        .body(body)
        .send()
        .await
        .unwrap();

        let text = res.text().await.unwrap();
        println!("{}", text);
        let json_res: Value = serde_json::from_str(&text).unwrap();

        let accounts_balances = json_res["balances"].as_object().unwrap_or_else( || {
                        panic!("Problem with RPC Node... {} \nIf this problem persists, please consider changing the RPC Node url in config.", text);
        });

        let mut opened_accounts: Vec<String> = vec![]; 

        for (account_address, balance_info) in accounts_balances {
                if balance_info["balance"] != "0" || balance_info["pending"] != "0" {
                        opened_accounts.push(account_address.clone());
                        if *stop_at_first {
                                return (true, opened_accounts);
                        }
                }
        }
        
        if opened_accounts.len() > 0 {
                return (true, opened_accounts);
        } 
        (false, opened_accounts)
}


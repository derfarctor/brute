#![allow(dead_code)]
#![allow(unused_imports)]
use std::thread;
use std::env;
use std::process;
use std::sync::{Arc, Mutex};
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;
use tokio::time::{sleep, Duration, Instant};
use itertools::Itertools;
use serde_json::{Value, json};
use toml;
use colour::{e_cyan, e_green, e_red_ln};
use std::error::Error;
mod utils;
use crate::utils::mnemonic;

type Terminator = Arc<Mutex<bool>>;
type KeysTested = Arc<AtomicUsize>;

const BATCH_SIZE: usize = 10000;
//const NODE_URL: &str = "https://app.natrium.io/api";
const NODE_URL: &str = "https://www.bitrequest.app:8020/?";
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
        let terminated = terminator.clone();

      
        let logger = thread::spawn(move || {
                threaded_logger(log_attempts, terminated, complexity);
        });
      
      /*
        let logger = tokio::spawn(async move {
                async_logger(log_attempts, terminated, complexity).await;
        });
         */

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
                                        println!("Time to kill process, first clean up threads and processes");
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
                println!("\nDid not find any opened account(s).");
        }
        thread::sleep(Duration::from_millis(4000));
        *terminator.lock().unwrap() = true;
        let _ = logger.join();
        //let _ = logger.await;
        println!("Logger resolved");
        println!("Ended");
}

// Implement break upon terminator set to true for both loggers
fn threaded_logger(log_attempts: KeysTested, terminated: Terminator, complexity: u64) {
        let mut last = 0;
        let mut last_log_length: usize = 0;
        loop {
                thread::sleep(Duration::from_millis(1000));
                if  *terminated.lock().unwrap() {
                        println!("Terminate recieved, ending logging process...");
                        break;
                }
                eprint!("{}", format!("\r{:>width$}", "", width=last_log_length));
                let attempts = log_attempts.load(atomic::Ordering::Relaxed);
                let log_msg = format!("\r{:.2}% done | {} mnemonics per second", 100.*(attempts as f64/complexity as f64), attempts-last);
                e_green!("{}", log_msg);
                last_log_length = log_msg.chars().count();
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
        for account in &batch {
                address_batch.push(account.address.clone());
        }
        let (found_account, account_addresses) = get_opened(address_batch, stop_at_first).await.unwrap_or_else(|error| {
                let mut update_found = terminator.lock().unwrap();
                *update_found = true;
                e_red_ln!("\nError in node request: {}", error);
                (false, vec![])
        });
        
        if found_account {
                // PROPERLY DISPLAY ACCOUNTS. If stop at first, just find single instance
                // else, make vec for account objects, then print all info OR WRITE TO FILE. Maybe add \n to avoid deletion by logger
                if *stop_at_first {
                        let mut update_found = terminator.lock().unwrap();
                        *update_found = true;
                        for account in &batch {
                                if account.address == account_addresses[0] {
                                        println!("\nAddress: {}", account_addresses[0]);
                                        println!("Seed: {}", hex::encode(account.seed_bytes));
                                }
                        }
                } else {
                        for account in &batch {
                                for account_address in &account_addresses {
                                        if &account.address == account_address {
                                                println!("\nAddress: {}", account_address);
                                                println!("Seed: {}", hex::encode(account.seed_bytes));
                                        }
                                }
                        }
                }
                return true;
        }
        false
}

async fn get_opened(address_batch: Vec<String>, stop_at_first: &bool) -> Result<(bool, Vec<String>), Box<dyn Error>> {
        let body_json = json!({
                "action":"accounts_balances",
                "accounts": address_batch
            });

        let body = body_json.to_string();

        let client = reqwest::Client::new();
        let res = client.post(NODE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await?;

        let text = res.text().await?;

        let json_res: Value = serde_json::from_str(&text)?;

        let accounts_balances = json_res["balances"].as_object().ok_or(format!("the node's response was not an accounts_balances object: {}", text))?;

        let mut opened_accounts: Vec<String> = vec![]; 

        for (account_address, balance_info) in accounts_balances {
                if balance_info["balance"] != "0" || balance_info["pending"] != "0" {
                        opened_accounts.push(account_address.clone());
                        if *stop_at_first {
                                return Ok((true, opened_accounts));
                        }
                }
        }
        
        if opened_accounts.len() > 0 {
                return Ok((true, opened_accounts));
        } 
        Ok((false, opened_accounts))
}

async fn async_logger(log_attempts: KeysTested, terminated: Terminator, complexity: u64) {
        let mut last = 0;
        let mut last_log_length: usize = 0;
        loop {
                sleep(Duration::from_millis(1000)).await;
                if  *terminated.lock().unwrap() {
                        println!("Terminate recieved, ending logging process...");
                        break;
                }
                eprint!("{}", format!("\r{:>width$}", "", width=last_log_length));
                let attempts = log_attempts.load(atomic::Ordering::Relaxed);
                let log_msg = format!("\r{:.2}% done | {} mnemonics per second", 100.*(attempts as f64/complexity as f64), attempts-last);
                e_green!("{}", log_msg);
                last_log_length = log_msg.chars().count();
                last = attempts;
        }
}
use std::{thread, process};
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use itertools::Itertools;
use colour::{e_red_ln};

use crate::mnemonic;
use crate::node;
use crate::logger;
use crate::config;

type Terminator = Arc<Mutex<bool>>;

struct Account {
        address: String,
        seed_bytes: [u8; 32],
}

pub async fn run(broken_mnemonic: [&str; 24], brute_config: config::Config) {

        let node_url = brute_config.node_url;
        let batch_size = brute_config.batch_size;

        // Yet to be implemented
        let req_cooldown = brute_config.request_cooldown;

        let stop_at_first = brute_config.stop_at_first;

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
                logger::threaded_logger(log_attempts, terminated, complexity);
        });
      
      /*
        let logger = tokio::spawn(async move {
                logger::async_logger(log_attempts, terminated, complexity).await;
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
                        if batch_accounts.len() == batch_size {
                                if *terminator.lock().unwrap() {
                                        println!("Time to kill process, first clean up threads and processes");
                                        let t = terminator.clone();
                                        cleanup(processes, logger, t).await;
                                        process::exit(1);
                                } else {
                                        let t = terminator.clone();
                                        let n = node_url.clone();
                                        let new_process = tokio::spawn(async move {
                                                return process(batch_accounts, &stop_at_first, t, n).await;
                                            });
                                        processes.push(new_process);
                                        batch_accounts = vec![];
                                }
                        }
                }
        }

        // Send last batch
        if batch_accounts.len() > 0 {
                if !*terminator.lock().unwrap() {
                        let t = terminator.clone();
                        let n = node_url.clone();
                        let new_process = tokio::spawn(async move {
                                return process(batch_accounts, &stop_at_first, t, n).await;
                            });
                        processes.push(new_process);
                }
        }
        let t = terminator.clone();
        cleanup(processes, logger, t).await;
}

async fn cleanup(processes: Vec<tokio::task::JoinHandle<bool>>, logger: std::thread::JoinHandle<()>, terminator: Terminator) {
        let mut found_one: bool = false;
        for process in processes {
                let final_check = process.await.unwrap();
                if final_check {
                        found_one = true;
                }
        }
        if found_one {
                println!("Finished! Found opened account(s).");
        } else {
                println!("\nFinished! Did not find any opened account(s).");
        }
        *terminator.lock().unwrap() = true;
        
        let _ = logger.join().unwrap_or_else(|error| {
                println!("Error ending logger thread: {:?}", error);
        });

        //let _ = logger.await;
        println!("Logger resolved");
        println!("Ended");
}

async fn process(batch: Vec<Account>, stop_at_first: &bool, terminator: Terminator, node_url: String) -> bool {
        let mut address_batch: Vec<String> = vec![];
        for account in &batch {
                address_batch.push(account.address.clone());
        }

        let node_url = node_url.as_str();
        let (found_account, account_addresses) = node::get_opened(node_url, address_batch, stop_at_first).await.unwrap_or_else(|error| {
                let mut update_found = terminator.lock().unwrap();
                if !*update_found {
                        *update_found = true;
                        e_red_ln!("\nError in node request. {}", error);
                }
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

/*
async fn send_batch(batch: Vec<Account>, stop_at_first: &bool, terminator: Terminator) -> Result<JoinHandle, bool> {
        if *terminator.lock().unwrap() {
                println!("Terminate true, ending processes");
                return Err(true);
                //process::exit(1);
        } else {
                let terminator = terminator.clone();
                let new_process = tokio::spawn(async move {
                        return process(batch_accounts, &stop_at_first, terminator).await;
                        });
                //processes.push(new_process);
                //batch_accounts = vec![];
        }
}
*/
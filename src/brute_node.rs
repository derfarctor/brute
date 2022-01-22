use std::{process, thread};
use tokio::time::Instant;
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use itertools::Itertools;
use colour::{e_red_ln, e_cyan_ln, e_grey_ln, e_red, e_magenta_ln};

use crate::mnemonic;
use crate::node;
use crate::logger;
use crate::config;

type Terminator = Arc<Mutex<bool>>;
type MnemonicsTested = Arc<AtomicUsize>;

struct Account {
        address: String,
        seed_bytes: [u8; 32],
}

pub async fn run(broken_mnemonic: [&str; 24], brute_config: config::Config) {

        let node_url = brute_config.node.node_url;
        let batch_size = brute_config.node.batch_size;
        let stop_at_first = brute_config.settings.stop_at_first;
        
        // Yet to be implemented
        let _req_cooldown = brute_config.node.request_cooldown;

        let (possibilities, unknown_indexes, complexity) = mnemonic::get_info(&broken_mnemonic);

        if complexity == 0 {
                e_red_ln!("The mnemonic has too many unknown parts to be calculated (Complexity greater than uint64 limit). Remove a complete unknown (X) or be more specific with sub-word options.");
                return;
        }
        e_cyan_ln!("Total mnemonic combinations to try: {}", complexity);

        let test_generator = possibilities.iter().map(|x| x.iter()).multi_cartesian_product();
        
        let mut test_mnemonic: [u16; 24] = mnemonic::get_test_mnemonic(&broken_mnemonic);
        let mut processes = vec![];
        let mut batch_accounts: Vec<Account> = vec![];
        
        let terminator = Arc::new(Mutex::new(false));
        let mnemonics_tested = Arc::new(AtomicUsize::new(0));
        

        let log_mnemonics = mnemonics_tested.clone();
        let terminated = terminator.clone();

      
        let logger = thread::spawn(move || {
                if brute_config.settings.stats_logging {
                        logger::threaded_logger(log_mnemonics, terminated, complexity);
                }
        });
      
        /* Async logging
        let logger = tokio::spawn(async move {
                logger::async_logger(log_attempts, terminated, complexity).await;
        });
         */

        let address_generator = mnemonic::AddressGenerator {
                prefix: String::from(brute_config.settings.address_prefix),
        };

        let start_time = Instant::now();
        
        for comb in test_generator {
                mnemonics_tested.fetch_add(1, atomic::Ordering::Relaxed);
                for i in 0..unknown_indexes.len() {
                        test_mnemonic[unknown_indexes[i]] = *comb[i];
                }
                let (seed_bytes, valid) = mnemonic::validate_mnemonic(&test_mnemonic);
                if valid {
                        let priv_key_bytes: [u8; 32] = mnemonic::get_private_key(&seed_bytes);
                        let pub_key_bytes: [u8; 32] = mnemonic::get_public_key(&priv_key_bytes);
                        let address: String = address_generator.get_address(&pub_key_bytes);
                        let account: Account = Account { address, seed_bytes };
                        batch_accounts.push(account);
                        if batch_accounts.len() == batch_size {
                                if *terminator.lock().unwrap() {
                                        let runtime = start_time.elapsed();
                                        let time_bruting = runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0;
                                        cleanup(processes, logger, mnemonics_tested, time_bruting).await;
                                        return;
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
        let runtime = start_time.elapsed();
        let time_bruting = runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0;
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
        *terminator.lock().unwrap() = true;
        cleanup(processes, logger, mnemonics_tested, time_bruting).await;
}

async fn cleanup(processes: Vec<tokio::task::JoinHandle<bool>>, logger: std::thread::JoinHandle<()>, mnemonics_tested: MnemonicsTested, time_bruting: f64) {
        let mut found_one: bool = false;
        for process in processes {
                let final_check = process.await.unwrap();
                if final_check {
                        found_one = true;
                        break;
                }
        }

        let _ = logger.join().unwrap_or_else(|error| {
                e_red_ln!("Error ending logger thread: {:?}", error);
        });

        if found_one {
                e_cyan_ln!("\nFinished! Found opened account(s).");
        } else {
                e_cyan_ln!("\nFinished! Did not find any opened account(s).");
        }
        let tested = mnemonics_tested.load(atomic::Ordering::Relaxed);
        e_grey_ln!("Tested: {} mnemonics in {:.2} seconds.\nAverage rate: {:.0} mnemonics per second.", tested, time_bruting, tested as f64 / time_bruting);
        

        /* Async Logging
        let _ = logger.await;
        */
}

async fn process(batch: Vec<Account>, stop_at_first: &bool, terminator: Terminator, node_url: String) -> bool {
        let mut address_batch: Vec<String> = vec![];
        for account in &batch {
                address_batch.push(account.address.clone());
        }

        let node_url = node_url.as_str();
        let (found_account, account_addresses) = node::get_opened(node_url, address_batch, stop_at_first).await.unwrap_or_else(|error| {
                e_red!("\nError in node request: {}", error);
                process::exit(1);
        });
        
        if found_account {
                // PROPERLY DISPLAY ACCOUNTS. If stop at first, just find single instance
                // else, make vec for account objects, then print all info OR WRITE TO FILE. Maybe add \n to avoid deletion by logger
                if *stop_at_first {
                        let mut update_found = terminator.lock().unwrap();
                        *update_found = true;
                        for account in &batch {
                                if account.address == account_addresses[0] {
                                        e_magenta_ln!("\n\nAddress: {}", account_addresses[0]);
                                        e_magenta_ln!("Seed: {}", hex::encode(account.seed_bytes));
                                }
                        }
                } else {
                        for account in &batch {
                                for account_address in &account_addresses {
                                        if &account.address == account_address {
                                                e_magenta_ln!("\nAddress: {}", account_address);
                                                e_magenta_ln!("Seed: {}", hex::encode(account.seed_bytes));
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
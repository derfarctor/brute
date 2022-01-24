use std::{thread, fs, process};
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex, atomic, atomic::AtomicUsize};
use tokio::time::{Instant, Duration};
use itertools::Itertools;
use colour::{e_red_ln, e_cyan_ln, e_grey_ln, e_magenta_ln};
use heed::{EnvOpenOptions, Database, flags::Flags};
use heed::types::*;

use crate::mnemonic;
use crate::logger;
use crate::config;

type MnemonicsTested = Arc<AtomicUsize>;
type Terminator = Arc<Mutex<bool>>;

struct ComputeParams {
        env: heed::Env, 
        tx: std::sync::mpsc::Sender<bool>,
        terminated: Terminator,
        test_mnemonic: [u16; 24], 
        possibilities: Vec<Vec<u16>>, 
        unknown_indexes: Vec<usize>, 
        mnemonics_tested: MnemonicsTested,
        address_prefix: String, 
        stop_at_first: bool,
}

pub async fn run(broken_mnemonic: [&str; 24], brute_config: config::Config) {
        let path = Path::new(&brute_config.ledger.ledger_path);
        let db_file_data = fs::metadata(path).unwrap_or_else(|error| {
                e_red_ln!("There was an error finding the ledger database file supplied to ledger_path in brute_config.toml: {:?}", error);
                process::exit(1);
        });
        
        let db_size: usize = db_file_data.len() as usize;

        if db_size == 0 {
                e_red_ln!("The ledger database file supplied to ledger_path in brute_config.toml contained no data. Please check it is correct.");
                process::exit(1);
        }

        let mut env_builder = EnvOpenOptions::new();
        unsafe {
                env_builder.flag(Flags::MdbNoSubDir);
            }
        env_builder.max_dbs(1)
        // Strange quirk from testing. Setting map_size lower than the size of the db allows 
        // the file size to remain as it should be size wise once the program ends, rather than
        // expanding to the default map_size of 128gb. This has only been tested on windows...

        // Update and explanation: LMDB lib feature: if map_size is set below actual db size, it is overwritten to the 
        // current size of the db. Perfectly acceptable for readonly operations as performed by brute.
        .map_size(4096);

        let env = env_builder.open(path).unwrap();

        let (possibilities, unknown_indexes, complexity) = mnemonic::get_info(&broken_mnemonic);

        if complexity == 0 {
                e_red_ln!("The mnemonic has too many unknown parts to be calculated (Complexity greater than uint64 limit). Remove a complete unknown (X) or be more specific with sub-word options.");
                return;
        }
        e_cyan_ln!("Total mnemonic combinations to try: {}", complexity);

        let terminator = Arc::new(Mutex::new(false));
        let mnemonics_tested = Arc::new(AtomicUsize::new(0));

        let log_mnemonics = mnemonics_tested.clone();

        let terminated = terminator.clone();
        let logger = thread::spawn(move || {
                if brute_config.settings.stats_logging {
                        logger::threaded_logger(log_mnemonics, terminated, complexity);
                }
        });

        let mut tracker = vec![];

        let start_time = Instant::now();
        if brute_config.ledger.multithreaded {
                // one thread per core for multithreading
                let mut cpus = num_cpus::get();
                if cpus < 2 {
                        e_red_ln!("Error: cant multithread with less than two cores");
                        process::exit(1);
                }
                if 2048%cpus != 0 {
                        while cpus > 2 && 2048%cpus != 0 {
                                cpus -= 1;
                        }
                }
                
                let threads = cpus; 
                let split_possibilities = mnemonic::get_split_possibilites(possibilities, threads);
                for i in 0..threads {

                        let (tx, rx) = mpsc::channel();

                        let params = ComputeParams {
                                env: env.clone(),
                                tx: tx,
                                terminated: terminator.clone(),
                                test_mnemonic: mnemonic::get_test_mnemonic(&broken_mnemonic),
                                possibilities: split_possibilities[i].clone(),
                                unknown_indexes: unknown_indexes.clone(),
                                mnemonics_tested: mnemonics_tested.clone(),
                                address_prefix: brute_config.settings.address_prefix.clone(),
                                stop_at_first: brute_config.settings.stop_at_first.clone(),
                        };

                        let new_compute = thread::spawn(move || {
                                return compute(params);
                        });
                        tracker.push((rx, new_compute));
                }
        } else {
                let (tx, rx) = mpsc::channel();
                let params = ComputeParams {
                        env: env,
                        tx: tx,
                        terminated: terminator.clone(),
                        test_mnemonic: mnemonic::get_test_mnemonic(&broken_mnemonic),
                        possibilities: possibilities,
                        unknown_indexes: unknown_indexes,
                        mnemonics_tested: mnemonics_tested.clone(),
                        address_prefix: brute_config.settings.address_prefix,
                        stop_at_first: brute_config.settings.stop_at_first.clone(),
                };
                let new_compute = thread::spawn(move || {
                        return compute(params);
                });
                tracker.push((rx, new_compute));
        }

        let mut found = false;
        
        'outer: while tracker.len() != 0 {
                let mut remaining = vec![];
                for (rx, join_handle) in tracker.into_iter() {
                        let finishing = rx.try_recv();
                        if !finishing.is_err() {
                                found = join_handle.join().unwrap();
                                if found && brute_config.settings.stop_at_first {
                                        break 'outer;
                                }
                        } else {
                                println!("{}", finishing.unwrap());
                                remaining.push((rx, join_handle));
                        }
                }
                tracker = remaining;
                thread::sleep(Duration::from_millis(10));
        }

        if !found {
                *terminator.lock().unwrap() = true;
        }

        let runtime = start_time.elapsed();
        let time_bruting = runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0;

        cleanup(logger, mnemonics_tested, time_bruting, found).await;
}

async fn cleanup(logger: std::thread::JoinHandle<()>, mnemonics_tested: MnemonicsTested, time_bruting: f64, found: bool) {
        let _ = logger.join().unwrap_or_else(|error| {
                e_red_ln!("Error ending logger thread: {:?}", error);
        });

        if found {
                e_cyan_ln!("\nFinished! Found opened account(s).");
        } else {
                e_cyan_ln!("\nFinished! Did not find any opened account(s).");
        }

        let tested = mnemonics_tested.load(atomic::Ordering::Relaxed);
        e_grey_ln!("Tested: {} mnemonics in {:.2} seconds.\nAverage rate: {:.0} mnemonics per second.", tested, time_bruting, tested as f64 / time_bruting);
        process::exit(0);
}

fn compute(mut params: ComputeParams) -> bool {

        let mut found_one = false;

        let test_generator = params.possibilities.iter().map(|x| x.iter()).multi_cartesian_product();
        
        let address_generator = mnemonic::AddressGenerator {
                prefix: params.address_prefix,
        };

        let db: Database<OwnedType<[u8; 32]>, ByteSlice> = params.env.create_database(Some("accounts")).unwrap();
        let rtxn = params.env.read_txn().unwrap();

        for comb in test_generator {
                params.mnemonics_tested.fetch_add(1, atomic::Ordering::Relaxed);
                for i in 0..params.unknown_indexes.len() {
                        params.test_mnemonic[params.unknown_indexes[i]] = *comb[i];
                }
                let (seed_bytes, valid) = mnemonic::validate_mnemonic(&params.test_mnemonic);
                if valid {
                        let priv_key_bytes: [u8; 32] = mnemonic::get_private_key(&seed_bytes);
                        let pub_key_bytes: [u8; 32] = mnemonic::get_public_key(&priv_key_bytes);

                        if db.get(&rtxn, &pub_key_bytes).unwrap() != None {
                                let address: String = address_generator.get_address(&pub_key_bytes);
                                e_magenta_ln!("\n\nAddress: {}", address);
                                e_magenta_ln!("Seed: {}", hex::encode(seed_bytes));
                                if params.stop_at_first {
                                        let mut terminated = params.terminated.lock().unwrap();
                                        if !*terminated {
                                                *terminated = true;
                                                let _ = params.tx.send(true).unwrap_or_else(|error| {
                                                        e_red_ln!("Worker thread had issue communicating with main thread: {}", error);
                                                });
                                        }
                                        return true;
                                } else {
                                        found_one = true;
                                }
                        }
                }
        }
        if !*params.terminated.lock().unwrap() {
                let _ = params.tx.send(true).unwrap_or_else(|error| {
                        e_red_ln!("Worker thread had issue communicating with main thread: {}", error);
                });
        }
        return found_one;

}

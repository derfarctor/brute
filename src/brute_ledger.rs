
use std::{thread};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use tokio::time::Instant;
use itertools::Itertools;
use colour::{e_red_ln, e_cyan_ln, e_grey_ln, e_red, e_magenta_ln};
use heed::{EnvOpenOptions, Database, flags::Flags};
use heed::types::*;

use crate::mnemonic;
use crate::logger;
use crate::config;

type MnemonicsTested = Arc<AtomicUsize>;

pub async fn run(broken_mnemonic: [&str; 24], brute_config: config::Config) {
        let path = Path::new(&brute_config.ledger.ledger_path);
        let db_file_data = fs::metadata(path).unwrap();
        let db_size: usize = db_file_data.len() as usize;

        if db_size == 0 {
                panic!("File size is zero...");
        }

        // one thread per core for multithreading
        let cpus = num_cpus::get();

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
        .map_size(db_size);

        let env = env_builder.open(path).unwrap();

        let stop_at_first = brute_config.settings.stop_at_first;

        let (possibilities, unknown_indexes, complexity) = mnemonic::get_info(&broken_mnemonic);

        if complexity == 0 {
                e_red_ln!("The mnemonic has too many unknown parts to be calculated (Complexity greater than uint64 limit). Remove a complete unknown (X) or be more specific with sub-word options.");
                return;
        }
        e_cyan_ln!("Total mnemonic combinations to try: {}", complexity);

        let test_generator = possibilities.iter().map(|x| x.iter()).multi_cartesian_product();
        
        let mut test_mnemonic: [u16; 24] = mnemonic::get_test_mnemonic(&broken_mnemonic);
        
        let terminator = Arc::new(Mutex::new(false));
        let mnemonics_tested = Arc::new(AtomicUsize::new(0));

        let log_mnemonics = mnemonics_tested.clone();
        let terminated = terminator.clone();

      
        let logger = thread::spawn(move || {
                if brute_config.settings.stats_logging {
                        logger::threaded_logger(log_mnemonics, terminated, complexity);
                }
        });

        let address_generator = mnemonic::AddressGenerator {
                prefix: String::from(brute_config.settings.address_prefix),
        };

        let start_time = Instant::now();
        
        // single threaded
        let db: Database<OwnedType<[u8; 32]>, ByteSlice> = env.create_database(Some("accounts")).unwrap();
        let rtxn = env.read_txn().unwrap();

        for comb in test_generator {
                mnemonics_tested.fetch_add(1, atomic::Ordering::Relaxed);
                for i in 0..unknown_indexes.len() {
                        test_mnemonic[unknown_indexes[i]] = *comb[i];
                }
                let (seed_bytes, valid) = mnemonic::validate_mnemonic(&test_mnemonic);
                if valid {
                        let priv_key_bytes: [u8; 32] = mnemonic::get_private_key(&seed_bytes);
                        let pub_key_bytes: [u8; 32] = mnemonic::get_public_key(&priv_key_bytes);

                        if db.get(&rtxn, &pub_key_bytes).unwrap() != None {
                                let address: String = address_generator.get_address(&pub_key_bytes);
                                e_magenta_ln!("\n\nAddress: {}", address);
                                e_magenta_ln!("Seed: {}", hex::encode(seed_bytes));
                                if stop_at_first {
                                        break;
                                }
                        }
                }
        }
        let runtime = start_time.elapsed();
        let time_bruting = runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0;

        *terminator.lock().unwrap() = true;
        cleanup(logger, mnemonics_tested, time_bruting).await;
}

async fn cleanup(logger: std::thread::JoinHandle<()>, mnemonics_tested: MnemonicsTested, time_bruting: f64) {
        let _ = logger.join().unwrap_or_else(|error| {
                e_red_ln!("Error ending logger thread: {:?}", error);
        });

        e_cyan_ln!("\nFinished!");

        let tested = mnemonics_tested.load(atomic::Ordering::Relaxed);
        e_grey_ln!("Tested: {} mnemonics in {:.2} seconds.\nAverage rate: {:.0} mnemonics per second.", tested, time_bruting, tested as f64 / time_bruting);
}

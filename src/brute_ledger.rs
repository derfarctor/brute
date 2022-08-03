use colour::{e_cyan_ln, e_grey_ln, e_magenta_ln, e_red_ln};
use heed::types::*;
use heed::{flags::Flags, Database, EnvOpenOptions};
use itertools::Itertools;
use std::path::Path;
use std::sync::{atomic, atomic::AtomicUsize, Arc, Mutex};
use std::{fs, process, thread};
use tokio::time::Instant;

use crate::config;
use crate::logger;
use crate::mnemonic;

type MnemonicsTested = Arc<AtomicUsize>;

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
    env_builder
        .max_dbs(1)
        // Strange quirk from testing. Setting map_size lower than the size of the db allows
        // the file size to remain as it should be size wise once the program ends, rather than
        // expanding to the default map_size of 128gb. This has only been tested on windows...
        // Update and explanation: LMDB lib feature: if map_size is set below actual db size, it is overwritten to the
        // current size of the db (http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5) - within bounds of multiple of page size?.
        // Perfectly acceptable for readonly operations as performed by brute.
        // Must be a multiple of system page size. Set to 4096 for now but page size can be pulled from crate if this causes errors on some os.
        // See https://docs.rs/page_size/latest/page_size/
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

    if brute_config.settings.multithreaded {
        // one thread per core for multithreading
        let mut cpus = num_cpus::get();
        if cpus < 2 {
            e_red_ln!("Error: cant multithread with less than two cores");
            process::exit(1);
        }
        if 2048 % cpus != 0 {
            while cpus > 2 && 2048 % cpus != 0 {
                cpus -= 1;
            }
        }

        let threads = cpus;
        let split_possibilities = mnemonic::get_split_possibilites(possibilities, threads);
        for i in 0..threads {
            let env_handle = env.clone();
            let address_prefix = brute_config.settings.address_prefix.clone();
            let stop_at_first = brute_config.settings.stop_at_first.clone();
            let unknown_indexes_copy = unknown_indexes.clone();
            let mnemonics_tested_handle = mnemonics_tested.clone();
            let possibilities = split_possibilities[i].clone();
            let test_mnemonic: [u16; 24] = mnemonic::get_test_mnemonic(&broken_mnemonic);
            let new_compute = tokio::task::spawn_blocking(move || {
                return compute(
                    env_handle,
                    test_mnemonic,
                    possibilities,
                    unknown_indexes_copy,
                    mnemonics_tested_handle,
                    address_prefix,
                    stop_at_first,
                );
            });
            tracker.push(new_compute);
        }
    } else {
        let stop_at_first = brute_config.settings.stop_at_first.clone();
        let mnemonics_tested_handle = mnemonics_tested.clone();
        let test_mnemonic: [u16; 24] = mnemonic::get_test_mnemonic(&broken_mnemonic);
        let new_compute = tokio::task::spawn_blocking(move || {
            return compute(
                env,
                test_mnemonic,
                possibilities,
                unknown_indexes,
                mnemonics_tested_handle,
                brute_config.settings.address_prefix,
                stop_at_first,
            );
        });
        tracker.push(new_compute);
    }
    let start_time = Instant::now();
    let mut found = false;
    if brute_config.settings.stop_at_first == true {
        let mut remaining = tracker;
        while remaining.len() != 0 && !found {
            let (result, _, new_remaining) = futures::future::select_all(remaining).await;
            remaining = new_remaining;
            found = result.unwrap();
        }
    } else {
        let results = futures::future::join_all(tracker).await;
        for result in results {
            if result.unwrap() == true {
                found = true;
                break;
            }
        }
    }

    let runtime = start_time.elapsed();
    let time_bruting = runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0;

    *terminator.lock().unwrap() = true;
    cleanup(logger, mnemonics_tested, time_bruting, found).await;
}

async fn cleanup(
    logger: std::thread::JoinHandle<()>,
    mnemonics_tested: MnemonicsTested,
    time_bruting: f64,
    found: bool,
) {
    let _ = logger.join().unwrap_or_else(|error| {
        e_red_ln!("Error ending logger thread: {:?}", error);
    });

    if found {
        e_cyan_ln!("\nFinished! Found opened account(s).");
    } else {
        e_cyan_ln!("\nFinished! Did not find any opened account(s).");
    }

    let tested = mnemonics_tested.load(atomic::Ordering::Relaxed);
    e_grey_ln!(
        "Tested: {} mnemonics in {:.2} seconds.\nAverage rate: {:.0} mnemonics per second.",
        tested,
        time_bruting,
        tested as f64 / time_bruting
    );
    process::exit(0);
}

fn compute(
    env: heed::Env,
    mut test_mnemonic: [u16; 24],
    possibilities: Vec<Vec<u16>>,
    unknown_indexes: Vec<usize>,
    mnemonics_tested: MnemonicsTested,
    address_prefix: String,
    stop_at_first: bool,
) -> bool {
    let mut found_one = false;

    let test_generator = possibilities
        .iter()
        .map(|x| x.iter())
        .multi_cartesian_product();

    let address_generator = mnemonic::AddressGenerator {
        prefix: address_prefix,
    };

    let db: Database<OwnedType<[u8; 32]>, ByteSlice> =
        env.create_database(Some("accounts")).unwrap();
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
                    return true;
                } else {
                    found_one = true;
                }
            }
        }
    }
    found_one
}

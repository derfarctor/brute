use tokio::time::{/*sleep,*/ Duration};
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use colour::{e_green};

use std::{thread};

type Terminator = Arc<Mutex<bool>>;
type MnemonicsTested = Arc<AtomicUsize>;

pub fn threaded_logger(log_mnemonics: MnemonicsTested, terminated: Terminator, complexity: u64) {
        let mut last_tested = 0;
        let mut last_log_length: usize = 0;
        loop {
                eprint!("{}", format!("\r{:>width$}", "", width=last_log_length));
                let tested = log_mnemonics.load(atomic::Ordering::Relaxed);
                let log_msg = format!("\r{:.2}% done | {} mnemonics per second", 100.*(tested as f64/complexity as f64), tested-last_tested);
                e_green!("{}", log_msg);
                last_log_length = log_msg.chars().count();
                last_tested = tested;
                thread::sleep(Duration::from_millis(1000));
                if  *terminated.lock().unwrap() {
                        break;
                }
        }
}

/*
async fn async_logger(log_mnemonics: MnemonicsTested, terminated: Terminator, complexity: u64) {
        let mut last_tested = 0;
        let mut last_log_length: usize = 0;
        loop {
                eprint!("{}", format!("\r{:>width$}", "", width=last_log_length));
                let tested = log_mnemonics.load(atomic::Ordering::Relaxed);
                let log_msg = format!("\r{:.2}% done | {} mnemonics per second", 100.*(tested as f64/complexity as f64), tested-last_tested);
                e_green!("{}", log_msg);
                last_log_length = log_msg.chars().count();
                last_tested = tested;
                sleep(Duration::from_millis(1000)).await;
                if  *terminated.lock().unwrap() {
                        break;
                }
        }
}
*/
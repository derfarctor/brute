use tokio::time::{sleep, Duration};
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use colour::{e_green};

use std::{thread};

type Terminator = Arc<Mutex<bool>>;
type KeysTested = Arc<AtomicUsize>;

pub fn threaded_logger(log_attempts: KeysTested, terminated: Terminator, complexity: u64) {
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

pub async fn async_logger(log_attempts: KeysTested, terminated: Terminator, complexity: u64) {
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
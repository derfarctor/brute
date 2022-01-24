use tokio::time::{/*sleep,*/ Instant, Duration};
use std::sync::{Arc, Mutex, atomic, atomic::AtomicUsize};
use colour::{e_green};

use std::{thread};

type Terminator = Arc<Mutex<bool>>;
type MnemonicsTested = Arc<AtomicUsize>;

pub fn threaded_logger(log_mnemonics: MnemonicsTested, terminated: Terminator, complexity: u64) {
        let start_time = Instant::now();
        loop {
                let tested = log_mnemonics.load(atomic::Ordering::Relaxed);
                let percentage = 100.*(tested as f64/complexity as f64);
                let runtime = start_time.elapsed();
                let per_second = (tested as f64) / (runtime.as_secs() as f64 + runtime.subsec_millis() as f64 / 1000.0);
                e_green!("\r{:>width$}\r{:.2}% done | {} mnemonics per second", "", percentage, per_second as u64, width=50);
                thread::sleep(Duration::from_millis(1000));
                if  *terminated.lock().unwrap() {
                        break;
                }
        }
}
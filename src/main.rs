//#![allow(dead_code)]
//#![allow(unused_imports)]
use std::{process, io};
use colour::{e_red_ln, e_grey_ln};

pub mod mnemonic;
pub mod node;
pub mod logger;
pub mod brute;
pub mod config;

#[tokio::main]
async fn main() {
        
        let brute_config = config::get_config();

        let mnemonic_copy = brute_config.mnemonic.to_owned();
        let mnemonic_tokens: Vec<&str> = mnemonic_copy.split_whitespace().collect();

        if mnemonic_tokens.len() != 24 {
                e_red_ln!("Error: brute_config.toml's mnemonic setting contained {} elements, rather than the expected 24. Please refer to brute_config.toml for advice formatting the mnemonic setting.", mnemonic_tokens.len());
                end_nicely();
        }

        let mut broken_mnemonic: [&str; 24] = Default::default();
        for i in 0..24 {
                broken_mnemonic[i] = mnemonic_tokens[i];
        }

        if !mnemonic::is_valid_broken_mnemonic(&broken_mnemonic) {
                end_nicely();
        }

        brute::run(broken_mnemonic, brute_config).await;
        end_nicely();
}

fn end_nicely() {
        e_grey_ln!("\nPress enter to continue...");
        io::stdin().read_line(&mut String::new()).unwrap();
        process::exit(0);
}
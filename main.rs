//#![allow(dead_code)]
//#![allow(unused_imports)]
use std::{process, env};
use colour::{e_red_ln};

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
                e_red_ln!("brute_config.toml's mnemonic setting contained {} elements, rather than the expected 24. Please refer to brute_config.toml for advice formatting the mnemonic setting.", mnemonic_tokens.len());
                process::exit(0);
        }

        let mut broken_mnemonic: [&str; 24] = Default::default();
        for i in 0..24 {
                broken_mnemonic[i] = mnemonic_tokens[i];
        }

        if !mnemonic::is_valid_broken_mnemonic(&broken_mnemonic) {
                    process::exit(0);
        }

        brute::run(broken_mnemonic, brute_config).await;
}
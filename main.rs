use std::env;
use std::process;
use itertools::Itertools;

mod utils;
use crate::utils::mnemonic;


//        
fn main() {
        let args: Vec<String> = env::args().collect();
        
        let config = Config::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                process::exit(0);
            });

        let broken_mnemonic = config.user_mnemonic;

        if !mnemonic::is_valid_broken_mnemonic(&broken_mnemonic) {
                    process::exit(0);
        }
        let (possibilities, unknown_indexes, complexity) = mnemonic::get_info(&broken_mnemonic);
        
        if complexity == 0 {
                println!("The mnemonic has too many unknown parts to be calculated (Complexity greater than uint64 limit). Remove a complete unknown (X) or be more specific with sub-word options.");
                process::exit(0);
        }

        println!("Complexity: {}", complexity);

        let test_generator = possibilities.iter().map(|x| x.iter()).multi_cartesian_product();
        let mut test_mnemonic: [u16; 24] = [0u16; 24];

        for i in 0..24 {
                if mnemonic::WORD_LIST.contains(&broken_mnemonic[i]) {
                        test_mnemonic[i] = mnemonic::wordlist_position(&broken_mnemonic[i]);
                }
        }

        let unknowns = unknown_indexes.len();

        for comb in test_generator {
                for i in 0..unknowns {
                        test_mnemonic[unknown_indexes[i]] = *comb[i];
                }
                let (seed_bytes, valid) = mnemonic::validate_mnemonic(&test_mnemonic);
                if valid {
                        let p_key_bytes: [u8, 32] = mnemonic::get_private_key(&seed_bytes);
                        let pub_key_bytes: [u8, 32] = mnemonic::get_public_key(&p_key_bytes);
                        let addr: String = mnemonic::get_address(&pub_key_bytes);
                        println!("{}", addr);
                }
        }
}

struct Config<'a> {
        user_mnemonic: [&'a str; 24],
    }

impl<'a> Config<'a> {
        fn new(args: &[String]) -> Result<Config, &'static str> {
                if args.len() == 25 {
                        let mnemonic_slice = &args[1..25];

                        let mut user_mnemonic: [&str; 24] = Default::default();
                        for i in 0..24 {
                                user_mnemonic[i] = mnemonic_slice[i].as_str();
                        }
                
                        Ok(Config { user_mnemonic })
                } else {
                        return Err("");
                }
        }
}



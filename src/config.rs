use std::{fs, process};
use std::io::ErrorKind;
use serde::Deserialize;
use colour::{e_green_ln, e_red_ln};
use toml;

#[derive(Deserialize)]
pub struct Config {
        pub mnemonic: String,
        pub settings: Settings,
        pub ledger: Ledger,
        pub node: Node
}

#[derive(Deserialize)]
pub struct Settings {
        pub address_prefix: String,
        pub stop_at_first: bool,
        pub stats_logging: bool
}

#[derive(Deserialize)]
pub struct Ledger {
        pub use_ledger: bool,
        pub ledger_path: String,
        pub multithreaded: bool
}

#[derive(Deserialize)]
pub struct Node {
        pub node_url: String,
        pub batch_size: usize,
        pub request_cooldown: f64,
}

const DEFAULT_CONFIG: &str = r#"# For a full explanation of each setting, please refer to the README.md

mnemonic = ""

[settings] # General brute settings
address_prefix = "nano_"
stop_at_first = true 
stats_logging = true

[ledger] # Settings used when running brute with a ledger data.ldb file. This is by far the fastest way to use brute.
use_ledger = false 
ledger_path = "" 
multithreaded = true

[node] # Settings used to run brute using a node. This is slow but does not require downloading the ledger database.
node_url = "https://app.natrium.io/api"
batch_size = 10000
request_cooldown = 0.5"#;

pub fn get_config() -> Config {
        let config_contents = fs::read_to_string("brute_config.toml").unwrap_or_else(|error| {
                if error.kind() == ErrorKind::NotFound {
                        e_green_ln!("Config file not found... creating brute_config.toml with defaults. Please run brute again after you have added your mnemonic to the config file.");
                        fs::write("brute_config.toml", DEFAULT_CONFIG).unwrap_or_else(|error| {
                                e_red_ln!("There was an issue creating brute_config.toml: {:?}", error);
                                process::exit(1);
                        });
                        process::exit(0);
                } else {
                        e_red_ln!("Error reading brute_config.toml: {:?}", error);
                        process::exit(1);
                }
        });

        let config: Config = toml::from_str(config_contents.clone().as_str()).unwrap_or_else(|error| {
                e_red_ln!("There was an issue parsing brute_config.toml: {:?}", error);
                process::exit(1);
        });
        config
}

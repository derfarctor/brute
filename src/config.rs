use colour::{e_green_ln, e_red_ln};
use serde::Deserialize;
use std::io::ErrorKind;
use std::{fs, process};
use toml;

#[derive(Deserialize)]
pub struct Config {
    pub mnemonic: String,
    pub settings: Settings,
    pub address: Address,
    pub ledger: Ledger,
    pub node: Node,
}

#[derive(Deserialize)]
pub struct Settings {
    pub mode: u8,
    pub address_prefix: String,
    pub stop_at_first: bool,
    pub stats_logging: bool,
    pub multithreaded: bool,
}

#[derive(Deserialize)]
pub struct Address {
    pub addresses: String,
}

#[derive(Deserialize)]
pub struct Ledger {
    pub ledger_path: String,
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
mode = 1
address_prefix = "nano_"
stop_at_first = true 
stats_logging = true
multithreaded = true

[address] # Settings used when running brute to find the mnemonic phrase of an account with known possible address(es)
addresses = "" # Add the addresses of the accounts you wish to search for here, separated by a ',' comma.

# Modes 2 and 3 (below) should ONLY be used if you DO NOT know the address of the account you are trying to brute force the mnemonic of.

[ledger] # Settings used when running brute with a ledger data.ldb file. Mode must be set to 2.
ledger_path = "" 

[node] # Settings used to run brute using a node. Mode must be set to 3. Multithreading won't work in this mode to avoid overloading public nodes.
node_url = "https://app.natrium.io/api" # Use https://kaliumapi.appditto.com/api as a default for banano
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

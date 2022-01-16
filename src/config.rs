use std::{fs};
use std::io::ErrorKind;
use serde::Deserialize;
use colour::{e_green_ln, e_red_ln};
use toml;

#[derive(Deserialize)]
pub struct Config {
        pub mnemonic: String,
        pub stop_at_first: bool,
        pub stats_logging: bool,
        pub address_prefix: String,
        pub node_url: String,
        pub batch_size: usize,
        pub request_cooldown: f64,
}

const DEFAULT_CONFIG: &str = r#"# Mnemonic
mnemonic = ""

# brute Settings
stop_at_first = true
stats_logging = true

# Node Settings
address_prefix = "nano_"
node_url = "https://app.natrium.io/api"
batch_size = 10000
request_cooldown = 0.5"#;

pub fn get_config() -> Config {
        let config_contents = fs::read_to_string("brute_config.toml").unwrap_or_else(|error| {
                if error.kind() == ErrorKind::NotFound {
                        e_green_ln!("Config file not found... creating brute_config.toml with defaults. Please run brute again after you have added your mnemonic to the config file.");
                        fs::write("brute_config.toml", DEFAULT_CONFIG).unwrap_or_else(|error| {
                                e_red_ln!("There was an issue creating brute_config.toml: {:?}", error);
                        });
                } else {
                        e_red_ln!("Error reading brute_config.toml: {:?}", error);
                }
                super::end_nicely();
                String::from("")
        });

        let config: Config = toml::from_str(config_contents.clone().as_str()).unwrap_or_else(|error| {
                e_red_ln!("There was an issue parsing brute_config.toml: {:?}", error);
                super::end_nicely();
                toml::from_str(DEFAULT_CONFIG).unwrap()
        });
        config
}

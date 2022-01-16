use std::{process, fs};
use std::io::ErrorKind;
use serde::Deserialize;
use toml;

#[derive(Deserialize)]
pub struct Config {
        pub mnemonic: String,
        pub stop_at_first: bool,
        pub node_url: String,
        pub batch_size: usize,
        pub request_cooldown: f64,
}

const DEFAULT_CONFIG: &str = r#"# brute Settings
mnemonic = ""
stop_at_first = true

# Node Settings
node_url = "https://app.natrium.io/api"
batch_size = 10000
request_cooldown = 0.5"#;

pub fn get_config() -> Config {
        let config_contents = fs::read_to_string("brute_config.toml").unwrap_or_else(|error| {
                if error.kind() == ErrorKind::NotFound {
                        println!("Config file not found... creating brute_config.toml with defaults.");
                        fs::write("brute_config.toml", DEFAULT_CONFIG).unwrap_or_else(|error| {
                                println!("There was an issue creating brute_config.toml: {:?}", error);
                                process::exit(1);
                        });
                        process::exit(0);
                } else {
                        println!("Error reading brute_config.toml: {:?}", error);
                        process::exit(1);
                }
        });

        let config: Config = toml::from_str(config_contents.clone().as_str()).unwrap_or_else(|error| {
                println!("There was an issue parsing brute_config.toml: {:?}", error);
                process::exit(1);
        });
        config
}

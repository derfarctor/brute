use std::error::Error;
use serde_json::{Value, json};

pub async fn get_opened(node_url: &str, address_batch: Vec<String>, stop_at_first: &bool) -> Result<(bool, Vec<String>), Box<dyn Error>> {
        let body_json = json!({
                "action":"accounts_balances",
                "accounts": address_batch
            });

        let body = body_json.to_string();

        let client = reqwest::Client::new();
        let res = client.post(node_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await?;

        let text = res.text().await?;

        let json_res: Value = serde_json::from_str(&text)?;

        let accounts_balances = json_res["balances"].as_object().ok_or(text)?;

        let mut opened_accounts: Vec<String> = vec![]; 

        for (account_address, balance_info) in accounts_balances {
                if balance_info["balance"] != "0" || balance_info["pending"] != "0" {
                        opened_accounts.push(account_address.clone());
                        if *stop_at_first {
                                return Ok((true, opened_accounts));
                        }
                }
        }
        
        if opened_accounts.len() > 0 {
                return Ok((true, opened_accounts));
        } 
        Ok((false, opened_accounts))
}
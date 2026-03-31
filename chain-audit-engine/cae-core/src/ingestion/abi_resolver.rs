pub async fn fetch_contract_abi(address: &str) -> eyre::Result<String> {
    let api_key = std::env::var("ETHERSCAN_API_KEY")?;
    let url = format!("https://api.etherscan.io/api?module=contract&action=getabi&address={}&apikey={}", address, api_key);
    let res: serde_json::Value = reqwest::get(url).await?.json().await?;
    if res["status"] == "1" { Ok(res["result"].as_str().unwrap().to_string()) }
    else { eyre::bail!("ABI not found") }
}
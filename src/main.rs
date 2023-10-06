use dotenv::dotenv;
use eyre::Result;
use std::env;
use uniswap_swap_monitor::run;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let provider_ws = format!(
        "wss://mainnet.infura.io/ws/v3/{}",
        env::var("INFURA_KEY").unwrap()
    );
    let contract_address = env::var("POOL_ADDRESS").unwrap();
    let db_path = env::var("DB_PATH").unwrap();

    run(&provider_ws, &contract_address, &db_path).await?;

    Ok(())
}

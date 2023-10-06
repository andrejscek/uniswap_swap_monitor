use ethers::{
    core::{
        abi::AbiDecode,
        types::{Address, Filter, Log, H256, I256, U256},
    },
    providers::{Middleware, Provider, StreamExt, Ws},
};
use eyre::Result;
use rusqlite::{params, Connection};
use std::cmp::PartialEq;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
struct LogData {
    amount0: I256,
    amount1: I256,
    sqrt_price: U256,
    liquidity: u128,
    tick: i32,
}

struct CombinedLog {
    tx_hash: H256,
    sender: Address,
    receiver: Address,
    data: LogData,
}

impl CombinedLog {
    fn new(tx_hash: Option<H256>, sender: Address, receiver: Address, data: LogData) -> Self {
        CombinedLog {
            tx_hash: tx_hash.unwrap_or_default(),
            sender,
            receiver,
            data,
        }
    }
}

fn initialize_database(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
        tx_hash TEXT,
        sender_address TEXT,
        receiver_address TEXT,
        amount0 TEXT,
        amount1 TEXT,  
        sqrt_price TEXT,
        liquidity TEXT,
        tick INTEGER
      )",
        [],
    )?;

    Ok(conn)
}

fn create_pool_filter(contract_address: &str) -> Filter {
    let pool_address = Address::from_str(contract_address).unwrap();
    Filter::new()
        .address(vec![pool_address])
        .event("Swap(address,address,int256,int256,uint160,uint128,int24)")
}

fn print_log(combined_log: &CombinedLog, log_data: &LogData) {
    println!(
        "new | tx_hash: {:?}, sender: {:?}, receiver: {:?}, amount0: {:?}, amount1: {:?}, sqrt_price: {:?}, liquidity: {:?}, tick: {:?}",
        combined_log.tx_hash,
        combined_log.sender,
        combined_log.receiver,
        log_data.amount0,
        log_data.amount1,
        log_data.sqrt_price,
        log_data.liquidity,
        log_data.tick
    );
}

async fn connect_to_provider(provider_ws: &str) -> Result<Arc<Provider<Ws>>> {
    let provider = Provider::<Ws>::connect(provider_ws).await?;
    Ok(Arc::new(provider))
}

async fn process_log(log: Log, conn: &Connection) -> Result<()> {
    let log_data = decode_log_data(&log.data)?;
    let combined_log = CombinedLog::new(
        log.transaction_hash,
        Address::from(log.topics[1]),
        Address::from(log.topics[2]),
        log_data.clone(),
    );
    insert_log(&conn, &combined_log)?;
    print_log(&combined_log, &log_data);
    Ok(())
}

async fn handle_logs(
    client: Arc<Provider<Ws>>,
    conn: &Connection,
    pool_filter: &Filter,
) -> Result<()> {
    let mut stream = client.subscribe_logs(pool_filter).await?;
    while let Some(log) = stream.next().await {
        process_log(log, conn).await?;
    }
    Ok(())
}

fn decode_log_data(data: &[u8]) -> Result<LogData> {
    let (amount0, amount1, sqrt_price, liquidity, tick): (I256, I256, U256, u128, i32) =
        AbiDecode::decode(data)?;

    Ok(LogData {
        amount0,
        amount1,
        sqrt_price,
        liquidity,
        tick,
    })
}

fn insert_log(conn: &Connection, combined_log: &CombinedLog) -> Result<()> {
    conn.execute(
        "INSERT INTO logs (tx_hash, sender_address, receiver_address, amount0, amount1, sqrt_price, liquidity, tick)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            format!("{:#x}", combined_log.tx_hash),
            format!("{:#x}", combined_log.sender),
            format!("{:#x}", combined_log.receiver),
            combined_log.data.amount0.to_string(),
            combined_log.data.amount1.to_string(),
            combined_log.data.sqrt_price.to_string(),
            combined_log.data.liquidity.to_string(),
            combined_log.data.tick,
        ],
    )?;
    Ok(())
}

pub async fn run(provider_ws: &str, contract_address: &str, db_path: &str) -> eyre::Result<()> {
    let conn = initialize_database(db_path)?;
    let pool_filter = create_pool_filter(contract_address);
    let client = connect_to_provider(provider_ws).await?;

    handle_logs(client, &conn, &pool_filter).await?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use ethers::types::Bytes;
    use std::path::Path;
    use tempdir::TempDir;

    struct TestTransactionValues {
        tx_hash: &'static str,
        sender: &'static str,
        receiver: &'static str,
        data: &'static str,
        topic0: &'static str,
        topic1: &'static str,
        topic2: &'static str,
        ammount0: &'static str,
        ammount1: &'static str,
        sqrt_price: &'static str,
        liquidity: &'static str,
        tick: i32,
    }

    fn create_test_transaction_vals() -> TestTransactionValues {
        TestTransactionValues {
            tx_hash: "0xe92955b4c46b38de18c1cdd58b06d49d45d6f9ca0906a86918f4cf20650683b4",
            sender: "0xe592427a0aece92de3edee1f18e0157c05861564",
            receiver: "0x4b7d6c3cea01f4d54a9cad6587da106ea39da1e6",
            topic0: "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",
            topic1: "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564",
            topic2: "0x0000000000000000000000004b7d6c3cea01f4d54a9cad6587da106ea39da1e6",
            data: "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffff0511b800000000000000000000000000000000000000000000000000240e540e2dc0042000000000000000000000000000000000000610413a1a7c814aa98ca36d09f8b000000000000000000000000000000000000000000000001c4846addbd259faf00000000000000000000000000000000000000000000000000000000000316ab",
            ammount0: "-263120000",
            ammount1: "162381653432074306",
            sqrt_price: "1967716719848838692609454179917707",
            liquidity: "32607304702662909871",
            tick: 202411,
        }
    }

    // Helper function to create a test log
    fn create_test_log() -> Log {
        let x = create_test_transaction_vals();

        Log {
            address: Address::zero(),
            transaction_hash: Some(H256::from_str(x.tx_hash).unwrap()),
            topics: vec![
                H256::from_str(x.topic0).unwrap(),
                H256::from_str(x.topic1).unwrap(),
                H256::from_str(x.topic2).unwrap(),
            ],
            data: Bytes::from_str(x.data).unwrap(),
            ..Default::default()
        }
    }

    #[test]
    fn test_combined_log_creation() {
        let x = create_test_transaction_vals();

        let tx_hash = H256::from_str(x.tx_hash).unwrap();
        let sender = Address::from_str(x.sender).unwrap();
        let receiver = Address::from_str(x.receiver).unwrap();
        let log_data = LogData {
            amount0: I256::from_dec_str(x.ammount0).unwrap(),
            amount1: I256::from_dec_str(x.ammount1).unwrap(),
            sqrt_price: U256::from_dec_str(x.sqrt_price).unwrap(),
            liquidity: x.liquidity.parse::<u128>().unwrap(),
            tick: x.tick,
        };
        let combined_log = CombinedLog::new(Some(tx_hash), sender, receiver, log_data.clone());

        assert_eq!(combined_log.tx_hash, tx_hash);
        assert_eq!(combined_log.sender, sender);
        assert_eq!(combined_log.receiver, receiver);
        assert_eq!(combined_log.data, log_data);
    }
    #[test]
    fn test_decode_log_data() {
        let x = create_test_transaction_vals();
        let bytes = Bytes::from_str(x.data).unwrap();
        let decoded = decode_log_data(&bytes).unwrap();

        let actual = LogData {
            amount0: I256::from_dec_str(x.ammount0).unwrap(),
            amount1: I256::from_dec_str(x.ammount1).unwrap(),
            sqrt_price: U256::from_dec_str(x.sqrt_price).unwrap(),
            liquidity: x.liquidity.parse::<u128>().unwrap(),
            tick: x.tick,
        };

        assert_eq!(decoded, actual);
    }

    #[tokio::test]
    async fn test_connect_to_provider() {
        let provider_ws = "wss://mainnet.infura.io/ws/v3/befb17eb176e41ceb879a05778423030";
        let result = connect_to_provider(&provider_ws).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_database() {
        let temp_dir = TempDir::new("tmptest").unwrap();
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();
        let result = initialize_database(&db_path);
        assert!(result.is_ok());
        assert!(Path::new(&db_path).exists());
    }

    #[tokio::test]
    async fn test_process_log() {
        // Create a temporary database for testing
        let temp_dir = TempDir::new("tmptest").unwrap();
        let db_path = temp_dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string();
        let conn = initialize_database(&db_path).unwrap();

        // Create a test log
        let test_log = create_test_log();

        // Process the test log
        let result = process_log(test_log.clone(), &conn).await;
        assert!(result.is_ok());

        // Retrieve the inserted log data from the database
        let mut stmt = conn
            .prepare("SELECT * FROM logs")
            .expect("Failed to prepare SQL statement");
        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, String>("tx_hash"),
                    row.get::<_, String>("sender_address"),
                    row.get::<_, String>("receiver_address"),
                    row.get::<_, String>("amount0"),
                    row.get::<_, String>("amount1"),
                    row.get::<_, String>("sqrt_price"),
                    row.get::<_, String>("liquidity"),
                    row.get::<_, i32>("tick"),
                ))
            })
            .expect("Failed to execute SQL query");

        // Check if the retrieved values match the expected values
        let expected = create_test_transaction_vals();
        assert_eq!(row.0.unwrap(), expected.tx_hash);
        assert_eq!(row.1.unwrap(), expected.sender);
        assert_eq!(row.2.unwrap(), expected.receiver);
        assert_eq!(row.3.unwrap(), expected.ammount0);
        assert_eq!(row.4.unwrap(), expected.ammount1);
        assert_eq!(row.5.unwrap(), expected.sqrt_price);
        assert_eq!(row.6.unwrap(), expected.liquidity);
        assert_eq!(row.7.unwrap(), expected.tick);
    }
}

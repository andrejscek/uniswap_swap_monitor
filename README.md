# Uniswap Swap Event Monitor

The Uniswap Swap Event Monitor is a Rust library that provides functionality of monitoring a UniSwap pool contract Swap events and storing them in a SQLite database.

## Features

The main program will first create a table called Logs in the database if it does not exist. Then it will start monitoring the Swap events of the Uniswap pool contract and store them in the database. When a new event is detected, the program will print the decoded event data to the console.
The logs events are stored in the database in the following format:

```sql
        tx_hash TEXT,
        sender_address TEXT,
        receiver_address TEXT,
        amount0 TEXT,
        amount1 TEXT,  
        sqrt_price TEXT,
        liquidity TEXT,
        tick INTEGER
```

The program is designed to monitor a single contract address, but can be modified in the future to monitor multiple contracts by modifying the create_pool_filter method.
The program will run until it is terminated by the user.

## Running the Application

To run the Uniswap Swap Event Monitor application, follow these steps:

1. Clone the repository to your local machine:

   ```shell
   git clone https://github.com/andrejscek/uniswap_swap_monitor.git
   ```

2. Navigate to the project directory:

   ```shell
   cd uniswap_swap_monitor
   ```

3. Set environment variables:

   - You need to set the following environment variables in a `.env` file or directly in your shell:
     - `INFURA_KEY`: Your Infura project ID
     - `POOL_ADDRESS`: The Uniswap pool contract address
     - `DB_PATH`: The output path to your SQLite database

4. Build and run the application:

   ```shell
   cargo run
   ```

## Running Tests

To run tests for the Ethereum Log Monitor application, use the following command:

```shell
cargo test
```

## License

This application is licensed under the MIT License

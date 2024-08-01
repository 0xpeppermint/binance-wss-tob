### binance-wss-tob


# Binance Top of Book Fetcher

This Rust program connects to the Binance WebSocket stream to retrieve the best real-time bid and ask prices (Top of Book) for any spot trading pair. It calculates the bid-ask spread in basis points. It prints the updated Top of Book values every 2 seconds on a separate thread, using Tokio async runtime and a mpsc channel to forward messages between two tasks.

## Features

- Connects to Binance's WebSocket stream for real-time spot trading data. Keeps the connection alive indefinitely.
- Calculates and prints the bid-ask spread in basis points.
- Prints the top bid and ask prices and their quantities at regular intervals.


## Requirements

- Rust 
- Cargo (comes with Rust)
- Stable Internet connection 

## Usage

1. **Clone the Repository**

   ```bash
   git clone https://github.com/yourusername/your-repo.git
   cd binance-wss-tob
   ```
   

2. **Build the Project**

   ```bash
   cargo build
   ```


3. **Run**

   ```bash
   cargo run
   ```

  Optionally, specify ticker with `-- ticker` and print interval with `-- print-interval` in seconds (defaults to BTC with 2 seconds interval)

   ```bash
   cargo run -- --ticker MKR --print-interval 2
   ```



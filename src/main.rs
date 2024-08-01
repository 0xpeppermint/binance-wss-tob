use clap::{Parser};
use tokio;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, tungstenite::protocol::frame::coding::CloseCode};
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;
use chrono::{DateTime, Utc};

/// A simple CLI tool to fetch and print Top of Book data from Binance.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The interval (in seconds) at which to print the Top of Book data.
    #[arg(long, default_value_t = 2)]
    print_interval: u64,

    /// The ticker symbol to use for the WebSocket stream.
    #[arg(long, default_value = "btc")]
    ticker: String,
}


#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let print_interval = Duration::from_secs(args.print_interval);

    // Construct the WebSocket URL using the specified ticker
    let ticker_lowercase = args.ticker.to_lowercase();
    let url = format!("wss://stream.binance.com/stream?streams={}usdt@bookTicker", ticker_lowercase);

    let (tx, rx) = mpsc::channel(500);

    tokio::spawn(async move {
        if let Err(e) = websocket_listener(&url, tx).await {
            eprintln!("WebSocket error: {}", e);
        }
    });

    tokio::spawn(async move {
        let _ = print_top_of_book(rx, print_interval).await;
    });

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Connects to the WebSocket and forwards messages to the provided channel
///
/// # Inputs
/// * `url` - WebSocket URL to connect to
/// * `tx` - Sender end of the channel to forward messages to
///
/// # Returns
/// Result indicating success or failure
async fn websocket_listener(url: &str, tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut ws_stream, _) = connect_async(url).await?;
    println!("WebSocket connected");

    loop {
        tokio::select! {
            message = ws_stream.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = tx.send(text).await {
                            eprintln!("Error sending message through channel: {}", e);
                        }
                    },
                    Some(Ok(Message::Ping(ping))) => {
                        if let Err(e) = ws_stream.send(Message::Pong(ping)).await {
                            eprintln!("Error responding to ping frame: {}", e);
                            return Err(Box::new(e));
                        }
                    },
                    Some(Ok(Message::Close(Some(frame)))) => {
                        println!("WebSocket closed: {:?}", frame);
                        if frame.code != CloseCode::Normal {
                            return Err(Box::new(tokio_tungstenite::tungstenite::Error::ConnectionClosed));
                        }
                    },
                    Some(Ok(_)) => {},
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        return Err(Box::new(e));
                    },
                    None => {
                        println!("WebSocket connection closed by server.");
                        return Ok(());
                    },
                }
            }
        }
    }
}

/// Processes incoming WebSocket messages and prints the top of the book at regular intervals
///
/// # Inputs
/// * `rx` - Receiver end of the channel for receiving messages
/// * `print_interval` - Duration to wait between prints
///
/// # Returns
/// Result indicating success or failure
async fn print_top_of_book(mut rx: mpsc::Receiver<String>, print_interval: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let mut best_bid = 0.0;
    let mut best_ask = 0.0;
    let mut quantity_bid = 0.0;
    let mut quantity_ask = 0.0;
    let mut ticker = String::new();

    if let Some(message) = rx.recv().await {
        if let Ok(json) = serde_json::from_str::<Value>(&message) {
            if let Some(data) = json.get("data") {
                best_bid = data.get("b").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                best_ask = data.get("a").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                quantity_bid = data.get("B").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                quantity_ask = data.get("A").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                ticker = data.get("s").and_then(|v| v.as_str()).unwrap_or("").to_string();
            }
        }   
    }

    let mut interval = interval(print_interval);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now: DateTime<Utc> = Utc::now();
                let spread_bps = if best_bid > 0.0 && best_ask > 0.0 {
                    (best_ask - best_bid) / best_bid * 10000.0
                } else {
                    0.0
                };
                println!("[SPOT][{}][{}] Top of Book:", ticker, now.to_rfc3339());
                println!("Bid: {:.6} @ {:.4}", quantity_bid, best_bid);
                println!("Ask: {:.6} @ {:.4}", quantity_ask, best_ask);
                println!("Spread bps: {:.8}", spread_bps);
                println!("---");
            }
            Some(message) = rx.recv() => {
                if let Ok(json) = serde_json::from_str::<Value>(&message) {
                    if let Some(data) = json.get("data") {
                        best_bid = data.get("b").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        best_ask = data.get("a").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        quantity_bid = data.get("B").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        quantity_ask = data.get("A").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                        ticker = data.get("s").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    }
                }   
            }
        }
    }
}

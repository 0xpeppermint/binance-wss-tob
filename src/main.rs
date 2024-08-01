//! This program connects to a wss stream on Binance to retrieve best real-time 
//! bid-ask quotes for the MKRUSDT spot pair along with the current spread. 
//! Retrieved values are processed and printed on a regular interval on a parallel thread

use tokio;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, tungstenite::protocol::frame::coding::CloseCode};
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;
use chrono::{DateTime, Utc};


const URL: &str = "wss://stream.binance.com/stream?streams=mkrusdt@bookTicker";
const PRINT_INTERVAL: Duration = Duration::from_secs(2);


// use tokio async runtime with multithreading
#[tokio::main(flavor = "multi_thread")] 

// spawns websocket listener and msg processor on a new thread each, kept alive forever 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let (tx, rx) = mpsc::channel(500);

    tokio::spawn(async move {
        if let Err(e) = websocket_listener(URL, tx).await {
            eprintln!("WebSocket error: {}", e);
        }
    });
    
    tokio::spawn(async move {
        let _ = print_top_of_book(rx).await;
    });
    
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Spins up a wss stream connection and forwards msgs to the provided channel. 
/// Sends Poing msgs to keep wss alive
///
/// # Inputs
/// * `url` - wss url to connect to.
/// * `tx` - sender end of the channel to forward msgs to
/// 
/// # Returns
/// A result indicating success or failure.
async fn websocket_listener(url: &str, tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut ws_stream, _) = connect_async(url).await?;
    println!("WebSocket connected");

    loop {
        tokio::select! {
            message = ws_stream.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => { // forward msgs to the channel 
                        if let Err(e) = tx.send(text).await {
                            eprintln!("Error sending message through channel: {}", e);
                        }
                    },
                    Some(Ok(Message::Ping(ping))) => { // send Point to keep websocket connection alive 
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


/// Processes incoming wss messages and prints the top of the book at regular intervals.
/// 
/// # Inputs
/// * `rx` - receiver end of the channel for receiving messages.
/// 
/// # Returns
/// A result indicating success or failure.
async fn print_top_of_book(mut rx: mpsc::Receiver<String>) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut print_interval = interval(PRINT_INTERVAL);

    loop {
        tokio::select! {
            _ = print_interval.tick() => {
                let now: DateTime<Utc> = Utc::now();
                let spread_bps = if best_bid > 0.0 && best_ask > 0.0 {
                    (best_ask - best_bid) / (best_bid) * 10000.0
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


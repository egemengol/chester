use anyhow::Context;
use v4_manager::StreamOrderBook;

mod core_structs;
mod v4_manager;
mod v4_messages;

use futures_util::StreamExt;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     // v4_manager::subscribe_to_orderbook(v4_messages::Market::EthUsd).await
//     let mut stream = StreamOrderBook::start(v4_messages::Market::EthUsd).await?;
//     let mut stream = stream.stream().await?;
//     while let Some(orderbook_json) = stream.next().await {
//         println!("{}", &orderbook_json);
//     }
//     Ok(())
// }

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

fn jsonify_market(t: &str) -> String {
    let mut jsonified = String::new();
    if !t.starts_with('"') {
        jsonified.push('"');
    }
    jsonified.push_str(t.trim_end());
    if !t.ends_with('"') {
        jsonified.push('"');
    }
    jsonified
}

async fn handle_socket(mut socket: WebSocket) {
    let got_first = socket.recv().await;
    let got_first_text = match got_first {
        None => return, // client disconnect
        Some(Err(e)) => panic!("{}", e),
        Some(Ok(Message::Text(t))) => t,
        Some(Ok(_)) => {
            panic!("Got non-text message while waiting for market");
        }
    };
    let got_first_text = jsonify_market(&got_first_text);

    let market: v4_messages::Market = serde_json::from_str(&got_first_text)
        .context("Could not parse market")
        .unwrap();

    let mut stream = StreamOrderBook::start(market)
        .await
        .context("Starting orderbook stream failed")
        .unwrap();
    let mut stream = stream
        .stream()
        .await
        .context("Streaming the orderbook stream failed")
        .unwrap();

    while let Some(orderbook_json) = stream.next().await {
        let send_result = socket.send(Message::Text(orderbook_json)).await;
        if let Err(_) = send_result {
            eprintln!("User disconnected");
            return;
        }
    }
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/ws", get(handler));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await.unwrap();
    eprintln!("Hit the websocket connection on ws://127.0.0.1:7878/ws");
    eprintln!("Send a text message like ETH-USD, then wait for orderbook snapshots.");
    axum::serve(listener, app).await.unwrap();
}

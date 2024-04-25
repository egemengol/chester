// use anyhow::Context;
// use serde::Deserialize;
// use v4_manager::StreamOrderBook;

use anyhow::Context;
use futures_util::StreamExt;
use serde::Deserialize;
use upstream::OrderBookStream;
use upstream_types::Market;

mod core_types;
mod upstream;
mod upstream_types;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use axum_extra::extract::Query;

#[derive(Deserialize, Debug)]
struct WSParams {
    #[serde(rename = "market")]
    markets: Vec<Market>,
}

async fn handler(ws: WebSocketUpgrade, Query(params): Query<WSParams>) -> Response {
    if params.markets.is_empty() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("No markets provided".into())
            .unwrap();
    }
    ws.on_upgrade(move |websocket| handle_socket(websocket, params.markets))
    // ws.on_upgrade(nofusshandlesocket)
}

// async fn nofusshandlesocket(mut socket: WebSocket) {
//     socket
//         .send(Message::Text("hi".to_string()))
//         .await
//         .expect("send");
//     socket.close().await.unwrap();
// }

async fn handle_socket(mut socket: WebSocket, markets: Vec<Market>) {
    let mut stream = OrderBookStream::subscribe(&markets)
        .await
        .context("subscribing to markets in handle_socket")
        .unwrap();

    while let Ok(orderbook_json) = stream
        .next()
        .await
        .context("stream should be unending")
        .unwrap()
    {
        let send_result = socket.send(Message::Text(orderbook_json)).await;
        if send_result.is_err() {
            eprintln!("User disconnected");
            return;
        }
    }
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(handler));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await.unwrap();
    eprintln!(
        "Hit the websocket connection like ws://127.0.0.1:7878/?market=ETH-USD&market=BTC-USD"
    );
    axum::serve(listener, app).await.unwrap();
}

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let mut stream = OrderBookStream::subscribe(&[Market::EthUsd, Market::BtcUsd]).await?;
//     while let Ok(orderbook_json) = stream.next().await.context("unending stream")? {
//         println!("{}", &orderbook_json);
//     }
//     Ok(())
// }

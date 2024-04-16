use v4_manager::StreamOrderBook;

mod core_structs;
mod v4_manager;
mod v4_messages;

use futures_util::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // v4_manager::subscribe_to_orderbook(v4_messages::Market::EthUsd).await
    let mut stream = StreamOrderBook::start(v4_messages::Market::EthUsd).await?;
    let mut stream = stream.stream().await?;
    while let Some(orderbook_json) = stream.next().await {
        println!("{}", &orderbook_json);
    }
    Ok(())
}

// use axum::{
//     extract::ws::{WebSocket, WebSocketUpgrade},
//     response::{IntoResponse, Response},
//     routing::get,
//     Router,
// };

// async fn handler(ws: WebSocketUpgrade) -> Response {
//     ws.on_upgrade(handle_socket)
// }

// async fn handle_socket(mut socket: WebSocket) {

//     while let Some(msg) = socket.recv().await {
//         let msg = if let Ok(msg) = msg {
//             msg
//         } else {
//             // client disconnected
//             return;
//         };

//         if socket.send(msg).await.is_err() {
//             // client disconnected
//             return;
//         }
//     }
// }

// #[tokio::main]
// async fn main() {
//     // build our application with a single route
//     let app = Router::new().route("/ws", get(handler));

//     // run our app with hyper, listening globally on port 3000
//     let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await.unwrap();
//     axum::serve(listener, app).await.unwrap();
// }

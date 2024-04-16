use anyhow::Context;
use tokio::net::TcpStream;
// const TESTNET_INDEXER_API_HOST: &str = "https://dydx-testnet.imperator.co";
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use futures_util::{
    future,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};

// const NETWORK_ID: &str = "dydx-testnet-4";
use crate::v4_messages;

const TESTNET_INDEXER_WS_HOST: &str = "wss://dydx-testnet.imperator.co/v4/ws";

async fn recv_connected_msg(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> anyhow::Result<v4_messages::Connected> {
    let got = read.next().await;
    let msg = match got {
        None => unimplemented!("Did not read the 'connected' message"),
        Some(Err(e)) => unimplemented!("Got err {:?}", e),
        Some(Ok(msg)) => msg,
    };
    let text = match msg {
        tokio_tungstenite::tungstenite::Message::Text(t) => t,
        _ => unimplemented!("Unknown incoming ws message type: {:?}", msg),
    };
    let connected: v4_messages::Connected =
        serde_json::from_str(&text).context("'connected' message must be valid")?;
    println!("Connected!");
    Ok(connected)
}

async fn send_subscribe_msg(
    write: &mut SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    market: v4_messages::Market,
) -> anyhow::Result<()> {
    let subscribe = v4_messages::Subscribe::new_for_market(&market);
    let subscribe_json = serde_json::to_string(&subscribe)?;
    write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            subscribe_json,
        ))
        .await?;
    println!(
        "Subscribed to market: {}",
        serde_json::to_string(&market).unwrap()
    );
    Ok(())
}

async fn print_incoming_orderbook_messages(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> anyhow::Result<()> {
    let fut = read.for_each(|msg| {
        let got_json = match msg {
            Err(e) => unimplemented!("received error during orderbook updates: {}", e),
            Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => t,
            Ok(_) => unimplemented!("received nontext message during orderbook updates"),
        };
        let got: v4_messages::OrderbookIncomingMessages = serde_json::from_str(&got_json)
            .expect("could not deserialize during orderbook incoming");
        println!("Got: {:?}", got);
        future::ready(())
    });
    fut.await;
    Ok(())
}

pub async fn subscribe_to_orderbook(market: v4_messages::Market) -> anyhow::Result<()> {
    let (stream, _) = connect_async(TESTNET_INDEXER_WS_HOST)
        .await
        .context("Failed to connect")?;

    let (mut write, mut read) = stream.split();

    let _ = recv_connected_msg(&mut read).await?;
    let _ = send_subscribe_msg(&mut write, market).await?;
    let _ = print_incoming_orderbook_messages(&mut read).await?;

    Ok(())
}

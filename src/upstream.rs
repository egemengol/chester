use std::collections::BTreeMap;

use anyhow::Context;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, Stream, StreamExt,
};

// const NETWORK_ID: &str = "dydx-testnet-4";
use crate::{
    core_types::OrderBookState,
    upstream_types::{self, Market},
};

const TESTNET_INDEXER_WS_HOST: &str = "wss://dydx-testnet.imperator.co/v4/ws";
const PROD_INDEXER_WS_HOST: &str = "wss://indexer.dydx.trade/v4/ws";

async fn recv_connected_msg(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> anyhow::Result<upstream_types::Connected> {
    let got = read.next().await;
    let msg = match got {
        None => Err(anyhow::anyhow!(
            "Upstream connection got closed gracefully before receiving the Connected message"
        )),
        Some(Err(e)) => Err(anyhow::anyhow!(e).context(
            "Upstream connection got closed abruptly before receiving the Connected message",
        )),
        Some(Ok(msg)) => Ok(msg),
    }?;
    let text = match msg {
        tokio_tungstenite::tungstenite::Message::Text(t) => t,
        _ => unimplemented!("Unknown incoming ws message type: {:?}", msg),
    };
    let connected: upstream_types::Connected =
        serde_json::from_str(&text).context("'connected' message must be valid")?;
    eprintln!("Connected to dydx!");
    Ok(connected)
}

async fn send_subscribe_msg(
    write: &mut SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    market: &upstream_types::Market,
) -> anyhow::Result<()> {
    let subscribe = upstream_types::Subscribe::new_for_market(market);
    let subscribe_json = serde_json::to_string(&subscribe)?;
    write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            subscribe_json,
        ))
        .await?;
    eprintln!(
        "Subscribed to market: {}",
        serde_json::to_string(&market).unwrap()
    );
    Ok(())
}

#[derive(Debug, Default)]
pub struct OrderBookFolder {
    orderbooks: BTreeMap<Market, OrderBookState>,
}

impl OrderBookFolder {
    pub fn consume_subscribed_msg(
        &mut self,
        subscribed: upstream_types::Subscribed,
    ) -> anyhow::Result<String> {
        let market = subscribed.market.clone();
        let orderbook = <upstream_types::Subscribed as Into<OrderBookState>>::into(subscribed);
        let serialized = serde_json::to_string(&orderbook)?;
        let _ = self.orderbooks.insert(market, orderbook);
        Ok(serialized)
    }

    pub fn consume_channel_batch_msg(
        &mut self,
        batch: upstream_types::ChannelBatchData,
    ) -> anyhow::Result<String> {
        let orderbook = self.orderbooks.get_mut(&batch.market).context(format!(
            "The orderbook for {:?} has not seen a snapshot yet, got delta update",
            batch.market
        ))?;
        batch
            .update_orderbook(orderbook)
            .context("updating orderbook in consume_channel_batch_msg")?;
        serde_json::to_string(&orderbook).context("Serializing orderbook has failed")
    }

    pub fn consume_orderbook_incoming_msg(
        &mut self,
        msg: upstream_types::OrderbookIncomingMessages,
    ) -> anyhow::Result<String> {
        match msg {
            upstream_types::OrderbookIncomingMessages::ChannelBatchData(batch) => {
                self.consume_channel_batch_msg(batch)
            }
            upstream_types::OrderbookIncomingMessages::Subscribed(subscribed) => {
                self.consume_subscribed_msg(subscribed)
            }
        }
    }
}

pub struct OrderBookStream {
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    folder: OrderBookFolder,
}

impl OrderBookStream {
    pub async fn subscribe(markets: &[Market]) -> anyhow::Result<Self> {
        let (stream, _) = connect_async(PROD_INDEXER_WS_HOST)
            .await
            .context("Failed to connect")?;

        let (mut write, mut read) = stream.split();

        let _ = recv_connected_msg(&mut read).await?;
        for m in markets.iter() {
            send_subscribe_msg(&mut write, m).await?;
        }
        Ok(Self {
            read,
            folder: OrderBookFolder::default(),
        })
    }
}

impl Stream for OrderBookStream {
    type Item = anyhow::Result<String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let frame = futures_util::ready!(self.read.poll_next_unpin(cx))
            .context("The stream should be unending")?;
        let payload_json = match frame {
            Err(e) => unimplemented!("received error during orderbook updates: {}", e),
            Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => t,
            Ok(_) => unimplemented!("received nontext message during orderbook updates"),
        };
        let message: upstream_types::OrderbookIncomingMessages =
            serde_json::from_str(&payload_json)?;
        let snapshot: String = self.folder.consume_orderbook_incoming_msg(message)?;
        std::task::Poll::Ready(Some(Ok(snapshot)))
    }
}

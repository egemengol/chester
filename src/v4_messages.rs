use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::core_structs::{self, OrderBookState};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub struct Connected {
    connection_id: String,
    message_id: usize,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum SocketChannel {
    #[serde(rename = "v4_orderbook")]
    Orderbook,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub enum Market {
    #[serde(rename = "ETH-USD")]
    EthUsd,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Offer {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct ContentPiece {
    pub asks: Option<Vec<Offer>>,
    pub bids: Option<Vec<Offer>>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Contents {
    Multi(Vec<ContentPiece>),
    Single(ContentPiece),
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case", rename = "subscribed")]
pub struct Subscribed {
    pub connection_id: String,
    pub message_id: usize,
    pub channel: SocketChannel,
    #[serde(rename = "id")]
    pub market: Market,
    pub contents: ContentPiece,
}

#[allow(clippy::from_over_into)]
impl Into<OrderBookState> for Subscribed {
    fn into(self) -> OrderBookState {
        let asks: Option<Vec<core_structs::Offer>> = self.contents.asks.map(|v4asks| {
            v4asks
                .into_iter()
                .map(|v4offer| core_structs::Offer {
                    price: v4offer.price,
                    size: v4offer.size,
                })
                .collect()
        });
        let bids: Option<Vec<core_structs::Offer>> = self.contents.bids.map(|v4bids| {
            v4bids
                .into_iter()
                .map(|v4offer| core_structs::Offer {
                    price: v4offer.price,
                    size: v4offer.size,
                })
                .collect()
        });
        OrderBookState::construct_from(
            asks.unwrap_or_default(),
            bids.unwrap_or_default(),
            self.message_id,
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct ChannelBatchData {
    pub connection_id: String,
    pub message_id: usize,
    pub channel: SocketChannel,
    #[serde(rename = "id")]
    pub market: Market,
    pub contents: Vec<ContentPiece>,
}

impl ChannelBatchData {
    pub fn update_orderbook(self, orderbook: &mut OrderBookState) -> anyhow::Result<()> {
        let mut asks: Vec<core_structs::Offer> = Vec::default();
        let mut bids: Vec<core_structs::Offer> = Vec::default();
        for piece in self.contents {
            if let Some(v4asks) = piece.asks {
                asks.extend(v4asks.into_iter().map(|v4offer| core_structs::Offer {
                    price: v4offer.price,
                    size: v4offer.size,
                }))
            }
            if let Some(v4bids) = piece.bids {
                bids.extend(v4bids.into_iter().map(|v4offer| core_structs::Offer {
                    price: v4offer.price,
                    size: v4offer.size,
                }))
            }
        }
        orderbook.update_with(asks, bids, self.message_id)
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderbookIncomingMessages {
    Subscribed(Subscribed),
    // Error,
    // ChannelData,
    ChannelBatchData(ChannelBatchData),
    // PING
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", rename = "subscribe")]
pub struct Subscribe {
    pub channel: SocketChannel,
    #[serde(rename = "id")]
    pub market: Market,
    pub batched: bool,
}

impl Subscribe {
    pub fn new_for_market(market: &Market) -> Self {
        Self {
            channel: SocketChannel::Orderbook,
            batched: true,
            market: market.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json::from_str;

    #[test]
    fn test_parse_connected() {
        let incoming = r#"{"type":"connected","connection_id":"9a75aff4-923a-4f43-9197-81eefceaacd1","message_id":0}"#;
        let connected: Connected = from_str(&incoming).expect("should be valid");
        assert_eq!(
            connected.connection_id,
            String::from_str("9a75aff4-923a-4f43-9197-81eefceaacd1").unwrap()
        );
        assert_eq!(connected.message_id, 0);
    }

    #[test]
    fn test_parse_subscribed() {
        let incoming = r#"{"type":"subscribed","connection_id":"9a75aff4-923a-4f43-9197-81eefceaacd1","message_id":1,"channel":"v4_orderbook","id":"ETH-USD","contents":{"bids":[{"price":"3040.6","size":"0.658"},{"price":"3009.8","size":"6.645"},{"price":"3000","size":"0.006"},{"price":"2600","size":"0.007"},{"price":"2567","size":"0.03"},{"price":"2556","size":"0.029"},{"price":"2000","size":"0.015"},{"price":"1000","size":"0.12"},{"price":"356","size":"0.05"},{"price":"334.3","size":"0.002"},{"price":"332.4","size":"0.03"},{"price":"256","size":"0.136"},{"price":"33","size":"0.303"},{"price":"15","size":"11.196"}],"asks":[{"price":"3073.1","size":"0.022"},{"price":"3076.1","size":"0.022"},{"price":"3079.2","size":"0.022"},{"price":"3102.1","size":"0.645"},{"price":"3132.7","size":"6.384"},{"price":"3560","size":"0.009"}]}}"#;
        let subscribed: Subscribed = from_str(incoming).expect("should be valid");
        assert_eq!(
            subscribed.connection_id,
            String::from_str("9a75aff4-923a-4f43-9197-81eefceaacd1").unwrap()
        );
        assert_eq!(subscribed.message_id, 1);
        assert_eq!(subscribed.channel, SocketChannel::Orderbook);
        assert_eq!(subscribed.market, Market::EthUsd);
        assert_eq!(subscribed.contents.asks.expect("asks exists").len(), 6);
        assert_eq!(subscribed.contents.bids.expect("bids exists").len(), 14);
    }

    #[test]
    fn test_parse_channel_batch_data() {
        let incoming = r#"{"type":"channel_batch_data","connection_id":"9a75aff4-923a-4f43-9197-81eefceaacd1","message_id":2,"id":"ETH-USD","channel":"v4_orderbook","version":"1.0.0","contents":[{"asks":[["3102.1","0"]]},{"asks":[["3101.4","0.645"]]},{"bids":[["3040.6","0"]]},{"bids":[["3040","0.658"]]}]}"#;
        let subscribed: ChannelBatchData = from_str(incoming).expect("should be valid");
        assert_eq!(
            subscribed.connection_id,
            String::from_str("9a75aff4-923a-4f43-9197-81eefceaacd1").unwrap()
        );
        assert_eq!(subscribed.message_id, 2);
        assert_eq!(subscribed.channel, SocketChannel::Orderbook);
        assert_eq!(subscribed.market, Market::EthUsd);
        assert_eq!(subscribed.contents.len(), 4);
        assert_eq!(
            subscribed.contents[3],
            ContentPiece {
                bids: Some(vec![Offer {
                    price: Decimal::from_str("3040").unwrap(),
                    size: Decimal::from_str("0.658").unwrap()
                }]),
                asks: None,
            }
        );
    }
}

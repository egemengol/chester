use rust_decimal::Decimal;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

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
struct Offer {
    price: Decimal,
    size: Decimal,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct ContentPiece {
    asks: Option<Vec<Offer>>,
    bids: Option<Vec<Offer>>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Contents {
    Multi(Vec<ContentPiece>),
    Single(ContentPiece),
}

#[derive(Deserialize, Debug)]
pub struct OrderbookBatchUpdate {
    pub connection_id: String,
    pub message_id: usize,
    pub channel: SocketChannel,
    #[serde(rename = "id")]
    pub market: Market,
    pub contents: Contents,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderbookIncomingMessages {
    Subscribed(OrderbookBatchUpdate),
    // Error,
    // ChannelData,
    ChannelBatchData(OrderbookBatchUpdate),
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
        let subscribed: OrderbookBatchUpdate = from_str(incoming).expect("should be valid");
        assert_eq!(
            subscribed.connection_id,
            String::from_str("9a75aff4-923a-4f43-9197-81eefceaacd1").unwrap()
        );
        assert_eq!(subscribed.message_id, 1);
        assert_eq!(subscribed.channel, SocketChannel::Orderbook);
        assert_eq!(subscribed.market, Market::EthUsd);
        if let Contents::Single(piece) = subscribed.contents {
            assert_eq!(piece.asks.expect("asks exists").len(), 6);
            assert_eq!(piece.bids.expect("bids exists").len(), 14);
        } else {
            panic!("Expected single contents")
        }
    }

    #[test]
    fn test_parse_channel_batch_data() {
        let incoming = r#"{"type":"channel_batch_data","connection_id":"9a75aff4-923a-4f43-9197-81eefceaacd1","message_id":2,"id":"ETH-USD","channel":"v4_orderbook","version":"1.0.0","contents":[{"asks":[["3102.1","0"]]},{"asks":[["3101.4","0.645"]]},{"bids":[["3040.6","0"]]},{"bids":[["3040","0.658"]]}]}"#;
        let subscribed: OrderbookBatchUpdate = from_str(incoming).expect("should be valid");
        assert_eq!(
            subscribed.connection_id,
            String::from_str("9a75aff4-923a-4f43-9197-81eefceaacd1").unwrap()
        );
        assert_eq!(subscribed.message_id, 2);
        assert_eq!(subscribed.channel, SocketChannel::Orderbook);
        assert_eq!(subscribed.market, Market::EthUsd);
        if let Contents::Multi(pieces) = subscribed.contents {
            assert_eq!(pieces.len(), 4);
            assert_eq!(
                pieces[3],
                ContentPiece {
                    bids: Some(vec![Offer {
                        price: Decimal::from_str("3040").unwrap(),
                        size: Decimal::from_str("0.658").unwrap()
                    }]),
                    asks: None,
                }
            );
        } else {
            panic!("Expected multi contents")
        }
    }
}

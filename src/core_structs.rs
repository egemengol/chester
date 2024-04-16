use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::ser::SerializeStruct;
use serde::Serialize;

#[derive(Debug)]
pub struct Offer {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug)]
pub struct OrderBookState {
    epoch: usize,
    pub asks: BTreeMap<Decimal, Decimal>,
    pub bids: BTreeMap<Decimal, Decimal>,
}

impl Serialize for OrderBookState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut out = serializer.serialize_struct("OrderBook", 2)?;
        let asks: Vec<(Decimal, Decimal)> = self
            .asks
            .iter()
            .map(|(price, size)| (*price, *size))
            .collect();
        out.serialize_field("asks", &asks)?;
        let bids: Vec<(Decimal, Decimal)> = self
            .bids
            .iter()
            .rev()
            .map(|(price, size)| (*price, *size))
            .collect();
        out.serialize_field("bids", &bids)?;
        out.end()
    }
}

impl OrderBookState {
    pub fn construct_from(asks: Vec<Offer>, bids: Vec<Offer>, epoch: usize) -> Self {
        let map_asks = asks
            .into_iter()
            .filter(|offer| offer.size != Decimal::ZERO)
            .map(|o| (o.price, o.size))
            .collect();
        let map_bids = bids
            .into_iter()
            .filter(|offer| offer.size != Decimal::ZERO)
            .map(|o| (o.price, o.size))
            .collect();
        Self {
            asks: map_asks,
            bids: map_bids,
            epoch,
        }
    }

    pub fn update_with(
        &mut self,
        asks: Vec<Offer>,
        bids: Vec<Offer>,
        epoch: usize,
    ) -> anyhow::Result<()> {
        if epoch <= self.epoch {
            anyhow::bail!(
                "Known epoch ({}) is more advanced than the given ({})",
                self.epoch,
                epoch
            )
        } else {
            self.epoch = epoch;
        }

        for o in asks.into_iter() {
            if o.size == Decimal::ZERO {
                let _ = self.asks.remove(&o.price);
            } else {
                let _ = self.asks.insert(o.price, o.size);
            }
        }

        for o in bids.into_iter() {
            if o.size == Decimal::ZERO {
                let _ = self.bids.remove(&o.price);
            } else {
                let _ = self.bids.insert(o.price, o.size);
            }
        }
        Ok(())
    }
}

mod core_structs;
mod v4_manager;
mod v4_messages;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    v4_manager::subscribe_to_orderbook(v4_messages::Market::EthUsd).await
}

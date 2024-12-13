use std::future::Future;

use alloy::eips::BlockNumberOrTag;
use alloy::pubsub::Subscription;
use alloy::{
    providers::{Provider, RootProvider},
    rpc::types::{Block, BlockTransactionsKind, Header},
    transports::{BoxTransport, TransportError},
};
use futures_util::{stream, StreamExt};
use log::info;

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),
    #[error("Block not found for hash: {0}")]
    BlockNotFound(String),
    #[error("Block not found for number: {0}")]
    BlockNotFoundForNumber(u64),
}

pub async fn subscribe_to_blocks<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    action: T,
) -> Result<(), SubscriptionError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    match provider.subscribe_blocks().await {
        Ok(subscription) => {
            process_block(provider, subscription, action).await?;
        }
        Err(_err) => {
            info!("Using HTTP provider, switching to watch_blocks instead.");
            try_block_watching(provider, action).await?;
        }
    }

    Ok(())
}

async fn try_block_watching<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    action: T,
) -> Result<(), SubscriptionError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    let poller = provider.watch_blocks().await?;
    let mut stream = poller.into_stream().flat_map(stream::iter);

    while let Some(block_hash) = stream.next().await {
        match provider
            .get_block_by_hash(block_hash, BlockTransactionsKind::Full)
            .await?
        {
            Some(block) => {
                info!("Received block number: {}", block.header.number);
                action(block).await;
            }
            None => {
                return Err(SubscriptionError::BlockNotFound(block_hash.to_string()));
            }
        }
    }
    Ok(())
}

async fn process_block<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    subscription: Subscription<Header>,
    action: T,
) -> Result<(), SubscriptionError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    let mut stream = subscription.into_stream();

    while let Some(header) = stream.next().await {
        info!("Received block number: {}", header.number);
        match provider
            .get_block_by_number(
                BlockNumberOrTag::Number(header.number),
                BlockTransactionsKind::Full,
            )
            .await?
        {
            Some(block) => {
                action(block).await;
            }
            None => {
                return Err(SubscriptionError::BlockNotFoundForNumber(header.number));
            }
        }
    }
    Ok(())
}

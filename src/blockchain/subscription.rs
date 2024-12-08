use std::future::Future;

use alloy::eips::BlockNumberOrTag;
use alloy::pubsub::Subscription;
use alloy::{
    providers::{Provider, RootProvider},
    rpc::types::{Block, BlockTransactionsKind, Header},
    transports::{BoxTransport, TransportError},
};
use futures_util::{stream, StreamExt};

pub async fn subscribe_to_blocks<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    action: T,
) -> Result<(), TransportError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    // Subscribe to block updates from the provider
    match provider.subscribe_blocks().await {
        Ok(subscription) => {
            process_block(provider, subscription, action).await?;
        }
        Err(_err) => {
            // Handle the case where PubSub is unavailable
            println!("Using HTTP provider, switching to watch_blocks instead.");
            watch_blocks(provider, action).await?;
        }
    }

    Ok(())
}

async fn watch_blocks<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    action: T,
) -> Result<(), TransportError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    let poller = provider.watch_blocks().await?;
    let mut stream = poller.into_stream().flat_map(stream::iter);

    // Process incoming blocks
    while let Some(block_hash) = stream.next().await {
        match provider
            .get_block_by_hash(block_hash, BlockTransactionsKind::Full)
            .await?
        {
            Some(block) => {
                println!("Received block number: {}", block.header.number);
                action(block).await; // Call the provided action on the block
            }
            None => {
                println!("No block found for hash: {block_hash}");
            }
        }
    }
    Ok(())
}

async fn process_block<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    subscription: Subscription<Header>,
    action: T,
) -> Result<(), TransportError>
where
    T: Fn(Block) -> Fut,
    Fut: Future<Output = ()>,
{
    let mut stream = subscription.into_stream();

    // Process incoming blocks
    Ok(while let Some(header) = stream.next().await {
        println!("Received block number: {}", header.number);
        match provider
            .get_block_by_number(
                BlockNumberOrTag::Number(header.number),
                BlockTransactionsKind::Full,
            )
            .await?
        {
            Some(block) => {
                action(block).await; // Call the provided action on the block
            }
            None => {
                println!("Block not found for number {}", header.number);
            }
        }
    })
}

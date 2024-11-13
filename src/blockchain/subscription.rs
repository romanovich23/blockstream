use std::future::Future;

use super::configuration::{EventFilter, EventSubscription};
use alloy::eips::BlockNumberOrTag;
use alloy::network::TransactionResponse;
use alloy::{
    providers::{Provider, RootProvider},
    rpc::types::{Block, BlockTransactionsKind, Log},
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
            let mut stream = subscription.into_stream();

            // Process incoming blocks
            while let Some(header) = stream.next().await {
                println!("Received block number: {}", header.number);
                match provider
                    .get_block_by_number(
                        BlockNumberOrTag::Number(header.number),
                        BlockTransactionsKind::Full,
                    )
                    .await?
                {
                    Some(block) => {
                        action(block); // Call the provided action on the block
                    }
                    None => {
                        println!("Block not found for number {}", header.number);
                    }
                }
            }
        }
        Err(_err) => {
            // Handle the case where PubSub is unavailable
            println!("Using HTTP provider, switching to watch_blocks instead.");
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
                        action(block); // Call the provided action on the block
                    }
                    None => {
                        println!("No block found for hash: {block_hash}");
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn process_transaction_logs<'a, T, Fut>(
    provider: &RootProvider<BoxTransport>,
    block: Block,
    subscriptions: &'a [EventSubscription],
    process_event_log: T,
) -> Result<(), TransportError>
where
    T: Fn(&'a EventFilter, Log) -> Fut,
    Fut: Future<Output = ()>,
{
    // Iterate over the transactions in the block
    for transaction in block.transactions.into_transactions() {
        if let Some(to) = TransactionResponse::to(&transaction) {
            // Check if the destination address is in the filters
            if let Some(event_filters) = subscriptions
                .iter()
                .find(|subscription| subscription.contract_address == to)
                .map(|subscription| &subscription.events)
            {
                println!("Found transaction to contract: {:?}", to);

                // Fetch the transaction receipt
                match provider
                    .get_transaction_receipt(transaction.tx_hash())
                    .await
                {
                    Ok(Some(tx_receipt)) => {
                        // Iterate over logs in the transaction receipt
                        for log in tx_receipt.inner.logs() {
                            println!("Found log: {log:?}");
                            // Check each log for event hashes
                            for event_filter in event_filters {
                                if log.topics().contains(&event_filter.hash) {
                                    println!(
                                        "Event found in transaction {}",
                                        transaction.tx_hash()
                                    );
                                    // Call the closure to process the event data
                                    process_event_log(event_filter, log.clone()).await;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        println!("No receipt found for transaction {}", transaction.tx_hash())
                    }
                    Err(err) => eprintln!("Error fetching transaction receipt: {}", err),
                }
            }
        }
    }
    Ok(())
}

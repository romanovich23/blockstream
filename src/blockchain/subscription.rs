use alloy::{
    providers::{Provider, RootProvider},
    pubsub::PubSubFrontend,
    rpc::types::{Block, BlockTransactionsKind, Log},
    transports::{BoxTransport, TransportError},
};
use futures_util::{stream, StreamExt};

use super::configuration::{EventFilter, EventSubscription};

pub async fn subscribe_to_blocks(
    provider: RootProvider<BoxTransport>,
    action: fn(Block) -> (),
) -> Result<(), TransportError> {
    // Subscribe to block updates from the provider
    match provider.subscribe_blocks().await {
        Ok(subscription) => {
            let mut stream = subscription.into_stream();

            // Process incoming blocks
            while let Some(block) = stream.next().await {
                println!("Received block number: {}", block.header.number);
                action(block); // Call the provided action on the block
            }
        }
        Err(err) => {
            // Check if the error message indicates that PubSub is unavailable
            if err.to_string().contains("PubsubUnavailable") {
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
            } else {
                // Handle other errors
                return Err(err);
            }
        }
    }

    Ok(())
}

pub async fn process_transaction_logs<F>(
    provider: RootProvider<PubSubFrontend>,
    block: Block,
    subscriptions: &[EventSubscription],
    process_event_log: F,
) -> Result<(), TransportError>
where
    F: Fn(&EventFilter, &Log),
{
    // Iterate over the transactions in the block
    for transaction in block.transactions.into_transactions() {
        if let Some(to) = transaction.to {
            // Check if the destination address is in the filters
            if let Some(events) = subscriptions
                .iter()
                .find(|suscription| suscription.contract_address == to)
                .map(|suscription| &suscription.events)
            {
                println!("Found transaction to contract: {:?}", to);

                // Fetch the transaction receipt
                match provider.get_transaction_receipt(transaction.hash).await {
                    Ok(Some(tx_receipt)) => {
                        // Iterate over logs in the transaction receipt
                        for log in tx_receipt.inner.logs() {
                            println!("Found log: {log:?}");
                            // Check each log for event hashes
                            for event in events {
                                if log.topics().contains(&event.hash) {
                                    println!("Event found in transaction {}", transaction.hash);
                                    // Call the closure to process the event data
                                    process_event_log(event, log); // Adjust based on actual log type
                                }
                            }
                        }
                    }
                    Ok(None) => println!("No receipt found for transaction {}", transaction.hash),
                    Err(err) => eprintln!("Error fetching transaction receipt: {}", err),
                }
            }
        }
    }
    Ok(())
}

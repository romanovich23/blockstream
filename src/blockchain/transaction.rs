use crate::blockchain::configuration::{EventFilter, EventSubscription};
use alloy::network::TransactionResponse;
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::{Block, Log};
use alloy::transports::{BoxTransport, TransportError};
use log::{error, info};
use std::future::Future;

pub async fn process_transaction_logs<T, Fut>(
    provider: &RootProvider<BoxTransport>,
    block: Block,
    subscriptions: &[EventSubscription],
    process_event_log: T,
) -> Result<(), TransportError>
where
    T: Fn(EventFilter, Log) -> Fut,
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
                // Fetch the transaction receipt
                match provider
                    .get_transaction_receipt(transaction.tx_hash())
                    .await
                {
                    Ok(Some(tx_receipt)) => {
                        // Iterate over logs in the transaction receipt
                        for log in tx_receipt.inner.logs() {
                            // Check each log for event hashes
                            for event_filter in event_filters {
                                if log.inner.topics().contains(&event_filter.hash) {
                                    info!(
                                        "Event {} found in transaction {}",
                                        event_filter.event_name,
                                        transaction.tx_hash()
                                    );
                                    // Call the closure to process the event data
                                    process_event_log(event_filter.clone(), log.clone()).await;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        info!("No receipt found for transaction {}", transaction.tx_hash())
                    }
                    Err(err) => error!("Error fetching transaction receipt: {}", err),
                }
            }
        }
    }
    Ok(())
}

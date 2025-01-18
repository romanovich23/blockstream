use crate::blockchain::configuration::{EventFilter, EventSubscription};
use alloy::consensus::Transaction;
use alloy::network::TransactionResponse;
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::{Block, Log};
use alloy::transports::{BoxTransport, TransportError};
use log::{error, info};
use std::future::Future;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),
    #[error("Transaction receipt not found for hash: {0}")]
    ReceiptNotFound(String),
}

pub struct EthereumTransactionProcessor {
    provider: Arc<RootProvider<BoxTransport>>,
    subscriptions: Vec<EventSubscription>,
}

impl EthereumTransactionProcessor {
    pub fn new(
        provider: Arc<RootProvider<BoxTransport>>,
        subscriptions: Vec<EventSubscription>,
    ) -> Self {
        Self {
            provider,
            subscriptions,
        }
    }

    pub async fn process_transaction_logs<T, Fut>(
        &self,
        block: Block,
        process_event_log: T,
    ) -> Result<(), TransactionError>
    where
        T: Fn(EventFilter, Log) -> Fut + Send + Sync,
        Fut: Future<Output = ()> + Send,
    {
        for transaction in block.transactions.into_transactions() {
            if let Some(to) = transaction.to() {
                if let Some(event_filters) = self
                    .subscriptions
                    .iter()
                    .find(|subscription| subscription.contract_address == to)
                    .map(|subscription| &subscription.events)
                {
                    match self
                        .provider
                        .get_transaction_receipt(transaction.tx_hash())
                        .await?
                    {
                        Some(tx_receipt) => {
                            for log in tx_receipt.inner.logs() {
                                for event_filter in event_filters {
                                    if log.inner.topics().contains(&event_filter.hash) {
                                        info!(
                                            "Event {} found in transaction {}",
                                            event_filter.event_name,
                                            transaction.tx_hash()
                                        );
                                        process_event_log(event_filter.clone(), log.clone()).await;
                                    }
                                }
                            }
                        }
                        None => {
                            info!("No receipt found for transaction {}", transaction.tx_hash());
                            return Err(TransactionError::ReceiptNotFound(
                                transaction.tx_hash().to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

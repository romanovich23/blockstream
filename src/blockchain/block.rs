use std::future::Future;
use std::sync::Arc;

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::B256;
use alloy::providers::FilterPollerBuilder;
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

#[trait_variant::make(BlockSubscriber: Send)]
pub trait LocalBlockSubscriber {
    async fn subscribe<T, Fut>(&self, callback_fn: T) -> Result<(), SubscriptionError>
    where
        T: Fn(Block) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}

pub struct EthereumBlockSubscriber {
    provider: Arc<RootProvider<BoxTransport>>,
}

impl EthereumBlockSubscriber {
    pub fn new(provider: Arc<RootProvider<BoxTransport>>) -> Self {
        Self { provider }
    }

    async fn process_pubsub_block<T, Fut>(
        &self,
        subscription: Subscription<Header>,
        callback_fn: T,
    ) -> Result<(), SubscriptionError>
    where
        T: Fn(Block) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut stream = subscription.into_stream();

        while let Some(header) = stream.next().await {
            info!("Received block number: {}", header.number);
            match self
                .provider
                .get_block_by_number(
                    BlockNumberOrTag::Number(header.number),
                    BlockTransactionsKind::Full,
                )
                .await?
            {
                Some(block) => {
                    callback_fn(block).await;
                }
                None => {
                    return Err(SubscriptionError::BlockNotFoundForNumber(header.number));
                }
            }
        }
        Ok(())
    }

    async fn process_poll_block<T, Fut>(
        &self,
        poller: FilterPollerBuilder<BoxTransport, B256>,
        action: T,
    ) -> Result<(), SubscriptionError>
    where
        T: Fn(Block) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut stream = poller.into_stream().flat_map(stream::iter);

        while let Some(block_hash) = stream.next().await {
            match self
                .provider
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
}

impl BlockSubscriber for EthereumBlockSubscriber {
    async fn subscribe<T, Fut>(&self, callback_fn: T) -> Result<(), SubscriptionError>
    where
        T: Fn(Block) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        match self.provider.subscribe_blocks().await {
            Ok(subscription) => {
                self.process_pubsub_block(subscription, callback_fn).await?;
            }
            Err(_err) => {
                info!("Using HTTP provider, switching to watch_blocks instead.");
                self.process_poll_block(self.provider.watch_blocks().await?, callback_fn)
                    .await?;
            }
        }

        Ok(())
    }
}

use super::configuration::Configuration;
use alloy::{
    providers::{ProviderBuilder, RootProvider},
    transports::{BoxTransport, TransportError},
};
use log::info;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),
}

pub async fn build_connection(
    config: &Configuration,
) -> Result<RootProvider<BoxTransport>, ConnectionError> {
    let url = config.network.url();
    info!("Connecting to network at URL: {}", url);

    ProviderBuilder::new()
        .on_builtin(&url)
        .await
        .map_err(ConnectionError::TransportError)
}

use alloy::{
    providers::{ProviderBuilder, RootProvider},
    transports::{BoxTransport, TransportError},
};

use super::configuration::Configuration;

pub async fn build_connection(
    config: &Configuration,
) -> Result<RootProvider<BoxTransport>, TransportError> {
    ProviderBuilder::new()
        .on_builtin(&config.network.url())
        .await
}

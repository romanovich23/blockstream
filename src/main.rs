use blockstream::blockchain::block::BlockSubscriber;
use blockstream::blockchain::{
    decoder::{Decoder, EthereumDecoder},
    transaction::EthereumTransactionProcessor,
};
use blockstream::{
    blockchain::{block::EthereumBlockSubscriber, connection},
    configuration::load_config,
    utils::logger::initialize_logger,
};
use log::{error, info};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if let Err(err) = initialize_logger() {
        error!("Failed to initialize logger: {}", err);
        return;
    }

    let config = match load_config(None) {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to load configuration: {}", err);
            return;
        }
    };

    for subscription in &config.subscriptions {
        info!("Configured subscription - {:?}", subscription);
    }

    match connection::build_connection(&config).await {
        Ok(connection) => {
            let connection = Arc::new(connection);
            let subscriber = EthereumBlockSubscriber::new(connection.clone());
            let tx_processor = Arc::new(EthereumTransactionProcessor::new(
                connection.clone(),
                config.subscriptions,
            ));

            if let Err(err) = subscriber
                .subscribe(move |block| {
                    let tx_processor = tx_processor.clone();
                    async move {
                        if let Err(err) = tx_processor
                            .process_transaction_logs(block, |event_filter, log| async move {
                                match EthereumDecoder::new(event_filter.data_types)
                                    .decode(&log.data().data)
                                {
                                    Ok(parameters) => {
                                        info!("Event data output: {:?}", parameters);
                                    }
                                    Err(err) => {
                                        error!("Error decoding event: {}", err);
                                    }
                                }
                            })
                            .await
                        {
                            error!("Error processing transaction logs: {}", err);
                        }
                    }
                })
                .await
            {
                error!("Error subscribing to blocks: {}", err);
            }
        }
        Err(err) => {
            error!("Error connecting to the blockchain: {}", err);
        }
    }
}

use blockstream::blockchain::decoder::{Decoder, EthereumDecoder};
use blockstream::{
    blockchain::{
        connection, subscription::subscribe_to_blocks, transaction::process_transaction_logs,
    },
    configuration::load_config,
    utils::logger::initialize_logger,
};
use log::{error, info};

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
            if let Err(err) = subscribe_to_blocks(&connection, |block| async {
                if let Err(err) = process_transaction_logs(
                    &connection,
                    block,
                    &config.subscriptions,
                    |event_filter, log| async move {
                        match EthereumDecoder::new()
                            .decode(&event_filter.data_types, &log.data().data)
                        {
                            Ok(parameters) => {
                                info!("Event data output: {:?}", parameters);
                            }
                            Err(err) => {
                                error!("Error decoding event: {}", err);
                            }
                        }
                    },
                )
                .await
                {
                    error!("Error processing transaction logs: {}", err);
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

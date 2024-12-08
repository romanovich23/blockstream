use blockstream::{
    blockchain::{
        connection, decode::decode_event, subscription::subscribe_to_blocks,
        transaction::process_transaction_logs,
    },
    configuration::load_config,
    logger::initialize_logger,
};
use log::{error, info};

#[tokio::main]
async fn main() {
    initialize_logger();
    let config = load_config(Option::None);
    for subscription in &config.subscriptions {
        info!("Configured subscription - {:}", subscription)
    }
    match connection::build_connection(&config).await {
        Ok(connection) => {
            match subscribe_to_blocks(&connection, |block| async {
                match process_transaction_logs(
                    &connection,
                    block,
                    &config.subscriptions,
                    |event_filter, log| async move {
                        match decode_event(&event_filter, &log) {
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
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error processing transaction logs: {}", err);
                    }
                }
            })
            .await
            {
                Ok(_) => {}
                Err(err) => {
                    error!("Error subscribing to blocks: {}", err);
                }
            }
        }
        Err(err) => {
            error!("Error connecting to the blockchain: {}", err);
        }
    }
}

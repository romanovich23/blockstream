use blockstream::{
    blockchain::{
        connection,
        decode::decode_event,
        subscription::{process_transaction_logs, subscribe_to_blocks},
    },
    config::load_config,
};

#[tokio::main]
async fn main() {
    let config = load_config(Option::None);
    match connection::build_connection(&config).await {
        Ok(connection) => {
            match subscribe_to_blocks(&connection, |block| async {
                println!("Block: {:?}", block);
                match process_transaction_logs(
                    &connection,
                    block,
                    &config.subscriptions,
                    |event_filter, log| async move {
                        match decode_event(event_filter, &log) {
                            Ok(parameters) => {
                                println!("Event: {:?}", event_filter);
                                println!("Parameters: {:?}", parameters);
                            }
                            Err(err) => {
                                eprintln!("Error decoding event: {}", err);
                            }
                        }
                    },
                )
                .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("Error processing transaction logs: {}", err);
                    }
                }
            })
            .await
            {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Error subscribing to blocks: {}", err);
                }
            }
        }
        Err(err) => {
            eprint!("Error connecting to the blockchain: {}", err);
        }
    }
}

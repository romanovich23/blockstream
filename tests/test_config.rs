use std::str::FromStr;

use alloy::primitives::Address;
use wevscan::config::load_config;

#[test]
fn test_load_config_env() {
    let config = load_config(Some("test".to_string()));
    assert_eq!(config.network.url(), "http://localhost:8545/eth");
    assert_eq!(config.subscriptions.len(), 2);
    assert_eq!(
        config.subscriptions[0].contract_address,
        Address::from_str("0x061b3e39A7f08F739641D31b9aD5795B3a34159f").unwrap()
    );
    assert_eq!(
        config.subscriptions[0].events[0].signature,
        "TransferExecuted(address,address)"
    );
    assert_eq!(
        config.subscriptions[1].contract_address,
        Address::from_str("0x27FcDf131c8401ac27955d6116a2E90f3293683a").unwrap()
    );
    assert_eq!(
        config.subscriptions[1].events[0].signature,
        "Approval(address,address)"
    );
}

#[test]
fn test_load_config_default() {
    let config = load_config(Option::None);
    assert_eq!(config.network.url(), "ws://localhost:8545");
    assert_eq!(config.subscriptions.len(), 2);
    assert_eq!(
        config.subscriptions[0].contract_address,
        Address::from_str("0x061b3e39A7f08F739641D31b9aD5795B3a34159f").unwrap()
    );
    assert_eq!(
        config.subscriptions[0].events[0].signature,
        "TransferExecuted(address,address)"
    );
    assert_eq!(
        config.subscriptions[1].contract_address,
        Address::from_str("0x27FcDf131c8401ac27955d6116a2E90f3293683a").unwrap()
    );
    assert_eq!(
        config.subscriptions[1].events[0].signature,
        "Approval(address,address)"
    );
}

#[test]
#[should_panic]
fn test_load_config_invalid_env() {
    load_config(Some("invalid".to_string()));
}

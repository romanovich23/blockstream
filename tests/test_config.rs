use std::str::FromStr;

use alloy::primitives::Address;
use blockstream::configuration::load_config;
use log::error;

#[test]
fn test_load_config_env() {
    match load_config(Some("test".to_string())) {
        Ok(config) => {
            assert_eq!(config.network.url(), "http://localhost:8545/eth");
            assert_eq!(config.subscriptions.len(), 2);
            assert_eq!(
                config.subscriptions[0].contract_address,
                Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap()
            );
            assert_eq!(
                config.subscriptions[0].events[0].signature,
                "DummyStructCreated(uint256,uint256,int256,bool,address,string,bytes32)"
            );
            assert_eq!(
                config.subscriptions[1].contract_address,
                Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap()
            );
            assert_eq!(
                config.subscriptions[1].events[0].signature,
                "DummyStructUpdated(uint256,uint256,int256,bool,address,string,bytes32)"
            );
        }
        Err(err) => {
            error!("Failed to load configuration: {}", err);
            panic!("Configuration loading failed: {}", err);
        }
    }
}

#[test]
fn test_load_config_default() {
    match load_config(None) {
        Ok(config) => {
            assert_eq!(config.network.url(), "ws://localhost:8545");
            assert_eq!(config.subscriptions.len(), 2);
            assert_eq!(
                config.subscriptions[0].contract_address,
                Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap()
            );
            assert_eq!(
                config.subscriptions[0].events[0].signature,
                "DummyStructCreated(uint256,uint256,int256,bool,address,string,bytes32)"
            );
            assert_eq!(
                config.subscriptions[1].contract_address,
                Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap()
            );
            assert_eq!(
                config.subscriptions[1].events[0].signature,
                "DummyStructUpdated(uint256,uint256,int256,bool,address,string,bytes32)"
            );
        }
        Err(err) => {
            error!("Failed to load configuration: {}", err);
            panic!("Configuration loading failed: {}", err);
        }
    }
}

#[test]
#[should_panic(expected = "Configuration loading failed: FileReadError")]
fn test_load_config_invalid_env() {
    if let Err(err) = load_config(Some("invalid".to_string())) {
        panic!("Configuration loading failed: {:?}", err);
    }
}

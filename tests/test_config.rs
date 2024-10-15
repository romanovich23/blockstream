use wevscan::config::load_config;

#[test]
fn test_load_config_env() {
    let config = load_config(Some("test".to_string()));
    assert_eq!(config.network.url, "http://localhost:/eth");
    assert_eq!(config.network.chain_id, 1);
    assert_eq!(config.filters.len(), 2);
    assert_eq!(config.filters[0].contract_address, "0x0");
    assert_eq!(
        config.filters[0].event_signature,
        "TransferExecuted(address,address)"
    );
    assert_eq!(config.filters[1].contract_address, "0x0");
    assert_eq!(
        config.filters[1].event_signature,
        "Approval(address,address)"
    );
}

#[test]
fn test_load_config_default() {
    let config = load_config(Option::None);
    assert_eq!(config.network.url, "http://localhost:/eth");
    assert_eq!(config.network.chain_id, 1);
    assert_eq!(config.filters.len(), 2);
    assert_eq!(config.filters[0].contract_address, "0x0");
    assert_eq!(
        config.filters[0].event_signature,
        "TransferExecuted(address,address)"
    );
    assert_eq!(config.filters[1].contract_address, "0x0");
    assert_eq!(
        config.filters[1].event_signature,
        "Approval(address,address)"
    );
}

#[test]
#[should_panic]
fn test_load_config_invalid_env() {
    load_config(Some("invalid".to_string()));
}

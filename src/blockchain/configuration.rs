use std::{
    fmt::{self, Display},
    str::FromStr,
};

use alloy::primitives::{keccak256, Address, FixedBytes};
use regex::Regex;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub network: Network,
    pub subscriptions: Vec<EventSubscription>,
}

impl Configuration {
    pub fn new(network: Network, subscriptions: Vec<EventSubscription>) -> Self {
        Self {
            network,
            subscriptions,
        }
    }
}

#[derive(Debug, Error)]
pub enum NetworkProtocolError {
    #[error("Invalid network protocol: {0}")]
    InvalidProtocol(String),
}

#[derive(Debug)]
pub enum NetworkProtocol {
    Http,
    Https,
    WebSocket,
    Ipc,
}

impl Display for NetworkProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkProtocol::Http => write!(f, "http"),
            NetworkProtocol::Https => write!(f, "https"),
            NetworkProtocol::WebSocket => write!(f, "ws"),
            NetworkProtocol::Ipc => write!(f, "ipc"),
        }
    }
}

impl<'de> Deserialize<'de> for NetworkProtocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "http" => Ok(NetworkProtocol::Http),
            "https" => Ok(NetworkProtocol::Https),
            "ws" => Ok(NetworkProtocol::WebSocket),
            "ipc" => Ok(NetworkProtocol::Ipc),
            _ => Err(de::Error::custom(NetworkProtocolError::InvalidProtocol(s))),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Network {
    pub protocol: NetworkProtocol,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Network {
    pub fn new(protocol: NetworkProtocol, host: String, port: u16, path: String) -> Self {
        Self {
            protocol,
            host,
            port,
            path,
        }
    }

    pub fn url(&self) -> String {
        if !self.path.is_empty() {
            format!(
                "{}://{}:{}/{}",
                self.protocol, self.host, self.port, self.path
            )
        } else {
            format!("{}://{}:{}", self.protocol, self.host, self.port)
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub contract_address: Address,
    pub events: Vec<EventFilter>,
}

impl EventSubscription {
    pub fn new(contract_address: Address, events: Vec<EventFilter>) -> Self {
        Self {
            contract_address,
            events,
        }
    }
}

impl Display for EventSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Contract: {}, Events: {:?}",
            self.contract_address, self.events
        )
    }
}

impl<'de> Deserialize<'de> for EventSubscription {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EventSubscriptionVisitor;

        impl<'de> Visitor<'de> for EventSubscriptionVisitor {
            type Value = EventSubscription;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct EventSubscription")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut contract_address = None;
                let mut events = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "contract_address" => {
                            if contract_address.is_some() {
                                return Err(de::Error::duplicate_field("contract_address"));
                            }
                            let value: &str = map.next_value()?;
                            contract_address = Some(Address::from_str(value).map_err(|_| {
                                de::Error::invalid_value(
                                    de::Unexpected::Str(value),
                                    &"a valid address",
                                )
                            })?);
                        }
                        "events" => {
                            if events.is_some() {
                                return Err(de::Error::duplicate_field("events"));
                            }
                            events = Some(map.next_value()?);
                        }
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let contract_address =
                    contract_address.ok_or_else(|| de::Error::missing_field("contract_address"))?;
                let events = events.ok_or_else(|| de::Error::missing_field("events"))?;

                Ok(EventSubscription::new(contract_address, events))
            }
        }

        deserializer.deserialize_map(EventSubscriptionVisitor)
    }
}

#[derive(Debug, Clone, Error)]
pub enum ParamTypeError {
    #[error("Unsupported data type: {0}")]
    UnsupportedDataType(String),
}

#[derive(Debug, Clone)]
pub enum ParamType {
    Address,
    UInt(usize),
    Int(usize),
    Bool,
    String,
    Bytes,
    FixedBytes(usize),
    Array(Box<ParamType>),
    Struct(Vec<ParamType>),
}

impl FromStr for ParamType {
    type Err = ParamTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "address" => Ok(ParamType::Address),
            "uint256" => Ok(ParamType::UInt(256)),
            "uint128" => Ok(ParamType::UInt(128)),
            "uint64" => Ok(ParamType::UInt(64)),
            "uint32" => Ok(ParamType::UInt(32)),
            "uint16" => Ok(ParamType::UInt(16)),
            "uint8" => Ok(ParamType::UInt(8)),
            "uint" => Ok(ParamType::UInt(256)),
            "int256" => Ok(ParamType::Int(256)),
            "int128" => Ok(ParamType::Int(128)),
            "int64" => Ok(ParamType::Int(64)),
            "int32" => Ok(ParamType::Int(32)),
            "int16" => Ok(ParamType::Int(16)),
            "int8" => Ok(ParamType::Int(8)),
            "int" => Ok(ParamType::Int(256)),
            "bool" => Ok(ParamType::Bool),
            "string" => Ok(ParamType::String),
            "bytes" => Ok(ParamType::Bytes),
            "bytes32" => Ok(ParamType::FixedBytes(32)),
            "bytes16" => Ok(ParamType::FixedBytes(16)),
            "bytes8" => Ok(ParamType::FixedBytes(8)),
            "bytes4" => Ok(ParamType::FixedBytes(4)),
            "bytes2" => Ok(ParamType::FixedBytes(2)),
            _ if s.starts_with('(') && s.ends_with(')') => {
                let inner = &s[1..s.len() - 1];
                let types: Vec<&str> = inner.split(',').collect();
                let mut struct_fields = Vec::new();
                for t in types {
                    struct_fields.push(ParamType::from_str(t.trim())?);
                }
                Ok(ParamType::Struct(struct_fields))
            }
            _ if s.ends_with("[]") => {
                let base_type = &s[..s.len() - 2];
                Ok(ParamType::Array(Box::new(ParamType::from_str(base_type)?)))
            }
            _ => Err(ParamTypeError::UnsupportedDataType(s.to_string())),
        }
    }
}

impl ParamType {
    pub fn name(&self) -> String {
        match self {
            ParamType::Address => "address".to_string(),
            ParamType::UInt(256) => "uint256".to_string(),
            ParamType::UInt(128) => "uint128".to_string(),
            ParamType::UInt(64) => "uint64".to_string(),
            ParamType::UInt(32) => "uint32".to_string(),
            ParamType::UInt(16) => "uint16".to_string(),
            ParamType::UInt(8) => "uint8".to_string(),
            ParamType::UInt(_) => "uint256".to_string(),
            ParamType::Int(256) => "int256".to_string(),
            ParamType::Int(128) => "int128".to_string(),
            ParamType::Int(64) => "int64".to_string(),
            ParamType::Int(32) => "int32".to_string(),
            ParamType::Int(16) => "int16".to_string(),
            ParamType::Int(8) => "int8".to_string(),
            ParamType::Int(_) => "int256".to_string(),
            ParamType::Bool => "bool".to_string(),
            ParamType::String => "string".to_string(),
            ParamType::Bytes => "bytes".to_string(),
            ParamType::FixedBytes(32) => "bytes32".to_string(),
            ParamType::FixedBytes(16) => "bytes16".to_string(),
            ParamType::FixedBytes(8) => "bytes8".to_string(),
            ParamType::FixedBytes(4) => "bytes4".to_string(),
            ParamType::FixedBytes(2) => "bytes2".to_string(),
            ParamType::FixedBytes(_) => "bytes".to_string(),
            ParamType::Array(inner_type) => format!("{}[]", inner_type.name()),
            ParamType::Struct(fields) => format!(
                "({})",
                fields
                    .iter()
                    .map(|f| f.name())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum EventFilterError {
    #[error("Invalid event signature format.")]
    InvalidSignatureFormat,
    #[error("Unsupported data type: {0}")]
    UnsupportedDataType(String),
}

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub signature: String,
    pub hash: FixedBytes<32>,
    pub event_name: String,
    pub data_types: Vec<ParamType>,
}

impl EventFilter {
    pub fn new(signature: String) -> Result<Self, EventFilterError> {
        if !Self::is_valid_signature_format(&signature) {
            return Err(EventFilterError::InvalidSignatureFormat);
        }

        let (event_name, data_types) = Self::extract_event_signature(&signature)?;

        Ok(Self {
            hash: keccak256(signature.as_bytes()),
            event_name,
            data_types,
            signature,
        })
    }

    fn is_valid_signature_format(event_signature: &str) -> bool {
        let re = Regex::new(r"^(\w+)\(([^)]+)\)$").unwrap();
        re.is_match(event_signature)
    }

    fn extract_event_signature(
        event_signature: &str,
    ) -> Result<(String, Vec<ParamType>), EventFilterError> {
        let re = Regex::new(r"^(\w+)\(([^)]+)\)$").unwrap();

        if let Some(captures) = re.captures(event_signature) {
            let event_name = captures[1].to_string();
            let types = &captures[2];

            let data_types: Result<Vec<ParamType>, EventFilterError> = types
                .split(',')
                .map(|s| s.trim())
                .map(|type_str| {
                    ParamType::from_str(type_str.trim())
                        .map_err(|_| EventFilterError::UnsupportedDataType(type_str.to_string()))
                })
                .collect();

            Ok((event_name, data_types?))
        } else {
            Err(EventFilterError::InvalidSignatureFormat)
        }
    }
}

impl<'de> Deserialize<'de> for EventFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        EventFilter::new(s).map_err(|err| serde::de::Error::custom(err))
    }
}

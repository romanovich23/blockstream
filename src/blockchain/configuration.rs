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

#[derive(Debug)]
pub enum NetworkProtocol {
    Http,
    Https,
    WebSocket,
    Ipc,
}

impl Display for NetworkProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "http" => Ok(NetworkProtocol::Http),
            "https" => Ok(NetworkProtocol::Https),
            "ws" => Ok(NetworkProtocol::WebSocket),
            "ipc" => Ok(NetworkProtocol::Ipc),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid network protocol: {}",
                s
            ))),
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

#[derive(Debug)]
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

impl<'de> Deserialize<'de> for EventSubscription {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Definir un Visitor para procesar el contenido
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
                            let value = map.next_value()?;
                            match Address::from_str(value) {
                                Ok(address) => contract_address = Some(address),
                                Err(_) => {
                                    return Err(de::Error::invalid_value(
                                        de::Unexpected::Str(value),
                                        &"a valid address",
                                    ))
                                }
                            }
                        }
                        "events" => {
                            if events.is_some() {
                                return Err(de::Error::duplicate_field("events"));
                            }
                            events = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignora campos adicionales si es necesario
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let contract_address =
                    contract_address.ok_or_else(|| de::Error::missing_field("contract_address"))?;
                let events = events.ok_or_else(|| de::Error::missing_field("events"))?;

                Ok(EventSubscription {
                    contract_address,
                    events,
                })
            }
        }

        // Llama a la función para deserializar utilizando el Visitor
        deserializer.deserialize_map(EventSubscriptionVisitor)
    }
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
    type Err = (); // O define un tipo de error más específico

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
                    match ParamType::from_str(t.trim()) {
                        Ok(data_type) => struct_fields.push(data_type),
                        Err(_) => return Err(()),
                    }
                }
                Ok(ParamType::Struct(struct_fields))
            }
            _ if s.ends_with("[]") => {
                let base_type = &s[..s.len() - 2];
                match ParamType::from_str(base_type) {
                    Ok(t) => Ok(ParamType::Array(Box::new(t))),
                    Err(_) => Err(()),
                }
            }
            _ => Err(()),
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

#[derive(Debug)]
pub struct EventFilter {
    pub signature: String,
    pub hash: FixedBytes<32>,
    pub event_name: String,
    pub data_types: Vec<ParamType>,
}

impl EventFilter {
    pub fn new(signature: String) -> Result<Self, String> {
        // First, validate the format of the event signature
        if !Self::is_valid_signature_format(&signature) {
            return Err("Invalid event signature format.".to_string());
        }

        // Extract event name and data types after validation
        let (event_name, data_types) = Self::extract_event_signature(&signature)?;

        Ok(Self {
            hash: keccak256(signature.as_bytes()),
            event_name,
            data_types,
            signature,
        })
    }

    // Function to validate the format of the event signature
    fn is_valid_signature_format(event_signature: &str) -> bool {
        let re = Regex::new(r"^(\w+)\(([^)]+)\)$").unwrap();
        re.is_match(event_signature)
    }

    // Function to extract the event signature
    fn extract_event_signature(event_signature: &str) -> Result<(String, Vec<ParamType>), String> {
        let re = Regex::new(r"^(\w+)\(([^)]+)\)$").unwrap(); // Ensure the signature starts and ends correctly

        if let Some(captures) = re.captures(event_signature) {
            let event_name = captures[1].to_string();
            let types = &captures[2];

            let data_types: Result<Vec<ParamType>, String> = types
                .split(',')
                .map(|s| s.trim())
                .map(|type_str| {
                    ParamType::from_str(type_str.trim())
                        .map_err(|_| format!("Unsupported data type: {}", type_str))
                })
                .collect();

            Ok((event_name, data_types?)) // Return event name and data types
        } else {
            Err("Event signature format is invalid.".to_string())
        }
    }
}

impl<'de> Deserialize<'de> for EventFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let result = EventFilter::extract_event_signature(&s);
        let (event_name, data_types) = match result {
            Ok((event_name, data_types)) => (event_name, data_types),
            Err(_) => {
                return Err(serde::de::Error::custom(format!(
                    "Invalid event signature: {}",
                    s
                )))
            }
        };

        Ok(EventFilter {
            hash: keccak256(s.as_bytes()),
            event_name,
            data_types,
            signature: s,
        })
    }
}

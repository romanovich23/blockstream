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
    pub events: Vec<Event>,
}

impl EventSubscription {
    pub fn new(contract_address: Address, events: Vec<Event>) -> Self {
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

#[derive(Debug)]
pub struct Event {
    pub signature: String,
    pub hash: FixedBytes<32>,
    pub event_name: String,
    pub data_types: Vec<String>,
}

impl Event {
    pub fn new(signature: String) -> Result<Self, String> {
        // First, validate the format of the event signature
        if !Self::is_valid_signature_format(&signature) {
            return Err("Invalid event signature format.".to_string());
        }

        // Extract event name and data types after validation
        let (event_name, data_types) = Self::extract_event_signature(&signature)?;

        // Validate that the data types are supported
        if !Self::validate_data_types(&data_types) {
            return Err("Some data types are not supported by Solidity.".to_string());
        }

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
    fn extract_event_signature(event_signature: &str) -> Result<(String, Vec<String>), String> {
        let re = Regex::new(r"^(\w+)\(([^)]+)\)$").unwrap(); // Ensure the signature starts and ends correctly

        if let Some(captures) = re.captures(event_signature) {
            let event_name = captures[1].to_string();
            let types = &captures[2];

            let data_types: Vec<String> = types.split(',').map(|s| s.trim().to_string()).collect();

            Ok((event_name, data_types))
        } else {
            Err("No match found.".to_string())
        }
    }

    // Function to validate that data types are supported by Solidity
    fn validate_data_types(data_types: &[String]) -> bool {
        let supported_types = [
            "address",
            "uint",
            "int",
            "bool",
            "string",
            "bytes",
            "address[]",
            "uint[]",
            "int[]",
            "bool[]",
            "string[]",
            "bytes[]",
        ];

        data_types
            .iter()
            .all(|data_type| supported_types.contains(&data_type.as_str()))
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let result = Event::extract_event_signature(&s);
        let (event_name, data_types) = match result {
            Ok((event_name, data_types)) => (event_name, data_types),
            Err(_) => {
                return Err(serde::de::Error::custom(format!(
                    "Invalid event signature: {}",
                    s
                )))
            }
        };

        Ok(Event {
            hash: keccak256(s.as_bytes()),
            event_name,
            data_types,
            signature: s,
        })
    }
}

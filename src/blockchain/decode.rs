use super::configuration::{EventFilter, ParamType};
use alloy::{
    primitives::{Address, Bytes, Signed},
    rpc::types::Log,
};

pub enum Parameter {
    Address(Address),
    Uint(usize),
    Int(isize),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    FixedBytes(Vec<u8>),
    FixedArray(Vec<Parameter>),
    Array(Vec<Parameter>),
    Tuple(Vec<Parameter>),
}

pub struct DecodeResult {
    pub parameter: Parameter,
    pub new_offset: usize,
}

pub fn decode_event(event: &EventFilter, log: &Log) -> Result<Vec<Parameter>, String> {
    let mut parameters: Vec<Parameter> = Vec::new();
    let data = log.data();
    let mut offset = 0;

    for data_type in &event.data_types {
        let result = decode_param(&data.data, data_type, offset)?;
        offset = result.new_offset;
        parameters.push(result.parameter);
    }

    Ok(parameters)
}

pub fn decode_param(
    data: &Bytes,
    param_type: &ParamType,
    offset: usize,
) -> Result<DecodeResult, String> {
    match param_type {
        ParamType::Address => {
            let slice = peek_32_bytes(data, offset)?;
            let address = Address::from_slice(&slice[12..]);
            let result = DecodeResult {
                parameter: Parameter::Address(address),
                new_offset: offset + 32,
            };
            Ok(result)
        }
        ParamType::UInt(_) => {
            let slice = peek_32_bytes(data, offset)?;
            let value = Signed::<256, 4>::try_from_be_slice(slice.as_slice())
                .expect("Data is not a valid signed integer");
            let result = DecodeResult {
                parameter: Parameter::Uint(value.as_usize()),
                new_offset: offset + 32,
            };
            Ok(result)
        }
        ParamType::Int(_) => {
            let slice = peek_32_bytes(data, offset)?;
            let value = Signed::<256, 4>::try_from_be_slice(slice.as_slice())
                .expect("Data is not a valid signed integer");
            let result = DecodeResult {
                parameter: Parameter::Int(value.as_isize()),
                new_offset: offset + 32,
            };
            Ok(result)
        }
        ParamType::Bool => {
            let slice = peek_32_bytes(data, offset)?;
            let value = slice[31] == 1;
            let result = DecodeResult {
                parameter: Parameter::Bool(value),
                new_offset: offset + 32,
            };
            Ok(result)
        }
        ParamType::String => {
            let dynamic_offset = as_usize(&peek_32_bytes(data, offset)?)?;
            let len = as_usize(&peek_32_bytes(data, dynamic_offset)?)?;
            let bytes = take_bytes(data, dynamic_offset + 32, len)?;
            let result = DecodeResult {
                parameter: Parameter::String(String::from_utf8_lossy(&bytes).into()),
                new_offset: offset + 32,
            };
            Ok(result)
        }
        ParamType::Bytes => todo!(),
        ParamType::FixedBytes(_) => todo!(),
        ParamType::Array(_) => todo!(),
        ParamType::Struct(_) => todo!(),
    }
}

fn peek(data: &[u8], offset: usize, len: usize) -> Result<&[u8], String> {
    if offset + len > data.len() {
        Err("Out of bounds".into())
    } else {
        Ok(&data[offset..(offset + len)])
    }
}

fn peek_32_bytes(data: &[u8], offset: usize) -> Result<[u8; 32], String> {
    peek(data, offset, 32).map(|x| {
        let mut out = [0u8; 32];
        out.copy_from_slice(&x[0..32]);
        out
    })
}

fn as_usize(slice: &[u8; 32]) -> Result<usize, String> {
    if !slice[..28].iter().all(|x| *x == 0) {
        return Err("Data is not a valid unsigned integer".into());
    }

    let result = ((slice[28] as usize) << 24)
        + ((slice[29] as usize) << 16)
        + ((slice[30] as usize) << 8)
        + (slice[31] as usize);

    Ok(result)
}

fn take_bytes(data: &[u8], offset: usize, len: usize) -> Result<Vec<u8>, String> {
    if offset + len > data.len() {
        return Err("Out of bounds".into());
    }
    Ok(data[offset..(offset + len)].to_vec())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use alloy::primitives::Address;
    use hex_literal::hex;

    #[test]
    fn test_decode_address() {
        let data = Bytes::from(hex!(
            "000000000000000000000000910ED056Ee239ae7e25f50F1E99255DC76d72E1C"
        ));
        let param_type = ParamType::Address;
        let offset = 0;

        let result = decode_param(&data, &param_type, offset).expect("Decodificación fallida");
        if let Parameter::Address(addr) = result.parameter {
            assert_eq!(
                addr,
                Address::from_str("0x910ED056Ee239ae7e25f50F1E99255DC76d72E1C").unwrap()
            );
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_uint() {
        let data = Bytes::from(hex!(
            "000000000000000000000000000000000000000000000000000000000000002a"
        ));
        let param_type = ParamType::UInt(256);
        let offset = 0;

        let result = decode_param(&data, &param_type, offset).expect("Decodificación fallida");
        if let Parameter::Uint(value) = result.parameter {
            assert_eq!(value, 42);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_int() {
        let data = Bytes::from(hex!(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd6"
        ));
        let param_type = ParamType::Int(256);
        let offset = 0;

        let result = decode_param(&data, &param_type, offset).expect("Decodificación fallida");
        if let Parameter::Int(value) = result.parameter {
            assert_eq!(value, -42);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_bool() {
        let data = Bytes::from(hex!(
            "0000000000000000000000000000000000000000000000000000000000000001"
        ));
        let param_type = ParamType::Bool;
        let offset = 0;

        let result = decode_param(&data, &param_type, offset).expect("Decodificación fallida");
        if let Parameter::Bool(value) = result.parameter {
            assert!(value);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_string() {
        let data = Bytes::from(hex!(
            "0000000000000000000000000000000000000000000000000000000000000020\
            000000000000000000000000000000000000000000000000000000000000000d\
            48656c6c6f2c20576f726c642100000000000000000000000000000000000000"
        ));
        let param_type = ParamType::String;
        let offset = 0;

        let result = decode_param(&data, &param_type, offset).expect("Decoding failed");
        if let Parameter::String(value) = result.parameter {
            assert_eq!(value, "Hello, World!");
        } else {
            panic!("Type of parameter incorrect");
        }
    }
}

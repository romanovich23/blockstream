use crate::blockchain::configuration::ParamType;
use alloy::primitives::{Address, Bytes, Signed};
use thiserror::Error;

#[derive(Debug, PartialEq)]
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
    Struct(Vec<Parameter>),
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("Out of bounds")]
    OutOfBounds,
    #[error("Invalid unsigned integer")]
    InvalidUnsignedInteger,
    #[error("Invalid signed integer")]
    InvalidSignedInteger,
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Memory allocation error")]
    MemoryAllocationError,
}

pub struct DecodeResult {
    pub parameter: Parameter,
    pub new_offset: usize,
}

pub trait Decoder {
    fn decode(&self, data: &Bytes) -> Result<Vec<Parameter>, DecodeError>;
    fn decode_parameter(
        &self,
        param_type: &ParamType,
        data: &Bytes,
        offset: usize,
    ) -> Result<DecodeResult, DecodeError>;
}

pub struct EthereumDecoder {
    pub param_types: Vec<ParamType>,
}

impl EthereumDecoder {
    pub fn new(param_types: Vec<ParamType>) -> Self {
        Self { param_types }
    }

    pub fn default() -> Self {
        Self {
            param_types: Vec::new(),
        }
    }

    fn decode_address(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let slice = peek_32_bytes(data, offset)?;
        let address = Address::from_slice(&slice[12..]);
        let result = DecodeResult {
            parameter: Parameter::Address(address),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_uint(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let slice = peek_32_bytes(data, offset)?;
        let value = Signed::<256, 4>::try_from_be_slice(slice.as_slice())
            .ok_or(DecodeError::InvalidSignedInteger)?;
        let result = DecodeResult {
            parameter: Parameter::Uint(value.as_usize()),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_int(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let slice = peek_32_bytes(data, offset)?;
        let value = Signed::<256, 4>::try_from_be_slice(slice.as_slice())
            .ok_or(DecodeError::InvalidSignedInteger)?;
        let result = DecodeResult {
            parameter: Parameter::Int(value.as_isize()),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_bool(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let slice = peek_32_bytes(data, offset)?;
        let value = slice[31] == 1;
        let result = DecodeResult {
            parameter: Parameter::Bool(value),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_string(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let dynamic_offset = as_usize(&peek_32_bytes(data, offset)?)?;
        let len = as_usize(&peek_32_bytes(data, dynamic_offset)?)?;
        let bytes = take_bytes(data, dynamic_offset + 32, len)?;
        let result = DecodeResult {
            parameter: Parameter::String(String::from_utf8(bytes)?),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_bytes(&self, data: &Bytes, offset: usize) -> Result<DecodeResult, DecodeError> {
        let dynamic_offset = as_usize(&peek_32_bytes(data, offset)?)?;
        let len = as_usize(&peek_32_bytes(data, dynamic_offset)?)?;
        let bytes = take_bytes(data, dynamic_offset + 32, len)?;
        let result = DecodeResult {
            parameter: Parameter::Bytes(bytes),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_fixed_bytes(
        &self,
        data: &Bytes,
        offset: usize,
        length: usize,
    ) -> Result<DecodeResult, DecodeError> {
        let bytes = take_bytes(data, offset, length)?;
        let result = DecodeResult {
            parameter: Parameter::FixedBytes(bytes),
            new_offset: offset + 32,
        };
        Ok(result)
    }

    fn decode_array(
        &self,
        data: &Bytes,
        offset: usize,
        t: &Box<ParamType>,
    ) -> Result<DecodeResult, DecodeError> {
        let len_offset = as_usize(&peek_32_bytes(data, offset)?)?;
        let len = as_usize(&peek_32_bytes(data, len_offset)?)?;

        let tail_offset = len_offset + 32;
        let tail = &data[tail_offset..];

        let mut parameters = vec![];
        parameters
            .try_reserve_exact(len)
            .map_err(|_| DecodeError::MemoryAllocationError)?;
        let mut new_offset = 0;

        for _ in 0..len {
            let res = self.decode_parameter(t, &Bytes::copy_from_slice(tail), new_offset)?;
            new_offset = res.new_offset;
            parameters.push(res.parameter);
        }

        let result = DecodeResult {
            parameter: Parameter::Array(parameters),
            new_offset: offset + 32,
        };

        Ok(result)
    }

    fn decode_struct(
        &self,
        data: &Bytes,
        offset: usize,
        t: &Vec<ParamType>,
    ) -> Result<DecodeResult, DecodeError> {
        let len = t.len();
        let mut parameters = Vec::with_capacity(len);
        let mut new_offset = offset;
        for param in t {
            let res = self.decode_parameter(param, data, new_offset)?;
            new_offset = res.new_offset;
            parameters.push(res.parameter);
        }

        let result = DecodeResult {
            parameter: Parameter::Struct(parameters),
            new_offset,
        };

        Ok(result)
    }
}

impl Decoder for EthereumDecoder {
    fn decode(&self, data: &Bytes) -> Result<Vec<Parameter>, DecodeError> {
        let mut parameters: Vec<Parameter> = Vec::new();
        let mut offset = 0;
        for param_type in &self.param_types {
            let result = self.decode_parameter(param_type, &data, offset)?;
            parameters.push(result.parameter);
            offset = result.new_offset;
        }
        Ok(parameters)
    }

    fn decode_parameter(
        &self,
        param_type: &ParamType,
        data: &Bytes,
        offset: usize,
    ) -> Result<DecodeResult, DecodeError> {
        match param_type {
            ParamType::Address => self.decode_address(data, offset),
            ParamType::UInt(_) => self.decode_uint(data, offset),
            ParamType::Int(_) => self.decode_int(data, offset),
            ParamType::Bool => self.decode_bool(data, offset),
            ParamType::String => self.decode_string(data, offset),
            ParamType::Bytes => self.decode_bytes(data, offset),
            ParamType::FixedBytes(size) => self.decode_fixed_bytes(data, offset, *size),
            ParamType::Array(t) => self.decode_array(data, offset, t),
            ParamType::Struct(t) => self.decode_struct(data, offset, t),
        }
    }
}

fn peek(data: &[u8], offset: usize, len: usize) -> Result<&[u8], DecodeError> {
    if offset + len > data.len() {
        Err(DecodeError::OutOfBounds)
    } else {
        Ok(&data[offset..(offset + len)])
    }
}

fn peek_32_bytes(data: &[u8], offset: usize) -> Result<[u8; 32], DecodeError> {
    peek(data, offset, 32).map(|x| {
        let mut out = [0u8; 32];
        out.copy_from_slice(&x[0..32]);
        out
    })
}

fn as_usize(slice: &[u8; 32]) -> Result<usize, DecodeError> {
    if !slice[..28].iter().all(|x| *x == 0) {
        return Err(DecodeError::InvalidUnsignedInteger);
    }

    let result = ((slice[28] as usize) << 24)
        + ((slice[29] as usize) << 16)
        + ((slice[30] as usize) << 8)
        + (slice[31] as usize);

    Ok(result)
}

fn take_bytes(data: &[u8], offset: usize, len: usize) -> Result<Vec<u8>, DecodeError> {
    if offset + len > data.len() {
        return Err(DecodeError::OutOfBounds);
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

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
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

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
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

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
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

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
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

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
        if let Parameter::String(value) = result.parameter {
            assert_eq!(value, "Hello, World!");
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_bytes() {
        let data = Bytes::from(hex!(
            "0000000000000000000000000000000000000000000000000000000000000020\
             0000000000000000000000000000000000000000000000000000000000000004\
             deadbeef00000000000000000000000000000000000000000000000000000000"
        ));
        let param_type = ParamType::Bytes;
        let offset = 0;

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
        if let Parameter::Bytes(value) = result.parameter {
            assert_eq!(value, vec![0xde, 0xad, 0xbe, 0xef]);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_fixed_bytes() {
        let data = Bytes::from(hex!(
            "deadbeef00000000000000000000000000000000000000000000000000000000"
        ));
        let param_type = ParamType::FixedBytes(4);
        let offset = 0;

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
        if let Parameter::FixedBytes(value) = result.parameter {
            assert_eq!(value, vec![0xde, 0xad, 0xbe, 0xef]);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_array() {
        let data = Bytes::from(hex!(
            "0000000000000000000000000000000000000000000000000000000000000020\
             0000000000000000000000000000000000000000000000000000000000000003\
             0000000000000000000000000000000000000000000000000000000000000001\
             0000000000000000000000000000000000000000000000000000000000000002\
             0000000000000000000000000000000000000000000000000000000000000003"
        ));
        let param_type = ParamType::Array(Box::new(ParamType::UInt(256)));
        let offset = 0;

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
        if let Parameter::Array(values) = result.parameter {
            let expected_values = vec![Parameter::Uint(1), Parameter::Uint(2), Parameter::Uint(3)];
            assert_eq!(values, expected_values);
        } else {
            panic!("Type of parameter incorrect");
        }
    }

    #[test]
    fn test_decode_struct() {
        let data = Bytes::from(hex!(
            "000000000000000000000000000000000000000000000000000000000000002a\
             0000000000000000000000000000000000000000000000000000000000000001"
        ));
        let param_type = ParamType::Struct(vec![ParamType::UInt(256), ParamType::Bool]);
        let offset = 0;

        let result = EthereumDecoder::default()
            .decode_parameter(&param_type, &data, offset)
            .expect("Decoding failed");
        if let Parameter::Struct(values) = result.parameter {
            let expected_values = vec![Parameter::Uint(42), Parameter::Bool(true)];
            assert_eq!(values, expected_values);
        } else {
            panic!("Type of parameter incorrect");
        }
    }
}

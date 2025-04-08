#![allow(warnings)]

use bendy::encoding::{Error, ToBencode};
use bendy::{serde::from_bytes, serde::to_bytes, value::Value};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};

#[derive(Debug, Clone, Serialize)]
pub enum OwnedValue {
    /// An owned byte string
    Str(String),
    /// A dictionary mapping byte strings to owned values
    Dict(BTreeMap<String, OwnedValue>),
    /// A signed integer
    Integer(i64),
    /// A list of owned values
    List(Vec<OwnedValue>),
}

fn safe_utf8_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (0x20..=0x7E).contains(&b) {
                // Printable ASCII character
                (b as char).to_string()
            } else {
                // Non-printable character, convert to hexadecimal representation
                format!("\\u{:04x}", b)
            }
        })
        .collect::<String>()
}

fn to_owned_value(value: Value) -> OwnedValue {
    match value {
        Value::Bytes(bytes) => OwnedValue::Str(safe_utf8_string(&bytes)),
        Value::Dict(dict) => {
            let mut owned_dict = BTreeMap::new();
            for (key, value) in dict {
                owned_dict.insert(safe_utf8_string(&key), to_owned_value(value));
            }
            OwnedValue::Dict(owned_dict)
        }
        Value::Integer(num) => OwnedValue::Integer(num),
        Value::List(list) => OwnedValue::List(list.into_iter().map(to_owned_value).collect()),
    }
}

pub fn decode_bencoded_string(contents: Vec<u8>) -> io::Result<BTreeMap<String, OwnedValue>> {
    let decoded: Value =
        from_bytes(&contents).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    match decoded {
        Value::Dict(map) => {
            let mut result_map = BTreeMap::new();
            for (key, value) in map {
                let key_string = safe_utf8_string(&key);
                result_map.insert(key_string, to_owned_value(value));
            }
            Ok(result_map)
        }
        _ => Err(io::Error::new(io::ErrorKind::Other, "Not a dictionary")),
    }
}

pub fn read_content(file_path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(file_path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

pub fn encode_info_field(file_path: &str) -> io::Result<String> {
    let contents = read_content(file_path)?;

    let decoded: Value =
        from_bytes(&contents).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if let Value::Dict(dict) = decoded {
        let info_field = dict
            .get("info".as_bytes())
            .expect("Required field missing: info");
        if let Value::Dict(info) = info_field {
            let bytes = info.to_bencode().unwrap();
            let bencode = unsafe { String::from_utf8_unchecked(bytes) };
            Ok(bencode.to_string())
        } else {
            Ok(String::from(""))
        }
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Not a dictionary"))
    }
}

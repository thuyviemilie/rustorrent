#![allow(warnings)]

use crate::bdecoder::OwnedValue;
use crate::parsing::File;
use crate::parsing::Info;
use std::collections::BTreeMap;
use std::io::Bytes;
use std::net::SocketAddrV4;

use crate::parsing::parse_metainfo;

use bendy::decoding::Error;
use curl::easy::Easy;
use std::io::{stdout, Write};

use crate::bdecoder::decode_bencoded_string;

use sha1::{Digest, Sha1};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::peers::Peers;

#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub compact: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    pub interval: usize,
    pub peers: Peers,
}

pub fn extract_info_field(dict: BTreeMap<String, OwnedValue>) -> BTreeMap<String, OwnedValue> {
    let info = dict.get("info").expect("Required field missing: info");
    if let OwnedValue::Dict(d) = info {
        d.clone()
    } else {
        panic!("Wrong type for field info");
    }
}

pub fn compute_length(info: &Info) -> usize {
    if info.length > 0 {
        info.length
    } else {
        let mut length: usize = 0;
        if let Some(files) = &info.files {
            for file in files {
                length += file.length;
            }
        }
        length
    }
}

pub async fn send_request(
    dict: BTreeMap<String, OwnedValue>,
    info_hash: [u8; 20],
) -> TrackerResponse {
    let meta_info = parse_metainfo(dict);

    let request = TrackerRequest {
        peer_id: String::from("-MB2025-100101070501"),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: compute_length(&meta_info.info),
        compact: 1,
    };

    let url_params = serde_urlencoded::to_string(&request).unwrap();
    let tracker_url = format!(
        "{}?{}&info_hash={}",
        meta_info.announce,
        url_params,
        &urlencode(&info_hash)
    );

    let response = reqwest::get(&tracker_url).await.unwrap();
    let response = response.bytes().await.unwrap();

    let tracker_response: TrackerResponse = serde_bencode::from_bytes(&response).unwrap();
    tracker_response
}

pub fn dump_peers(tracker_reponse: TrackerResponse) -> () {
    for peer in &tracker_reponse.peers.0 {
        println!("{}:{}", peer.ip(), peer.port());
    }
}

fn urlencode(t: &[u8]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

#![allow(warnings)]

mod bdecoder;
mod download;
mod parsing;
mod peers;
mod piece;
mod tracker;

use bdecoder::decode_bencoded_string;
use bdecoder::read_content;
use clap::{command, Arg, ArgAction, ArgMatches};
use futures_util::{SinkExt, StreamExt};
use peers::handshake_peer;
use peers::Handshake;
use peers::Message;
use peers::MessageFrame;
use peers::MessageTag;
use peers::Piece;
use peers::Request;
use std::fs;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::path::PathBuf;

use sha1::{Digest, Sha1};

use std::str::FromStr;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::time;
use tokio::time::Duration;
use tracker::compute_length;
use tracker::dump_peers;
use tracker::TrackerRequest;
use tracker::TrackerResponse;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::io::{self, Read};

use crate::{bdecoder::encode_info_field, tracker::send_request};

pub const BLOCK_MAX: usize = 1 << 14;

#[tokio::main]
async fn main() {
    let matches: ArgMatches = command!()
        .about("Rustorrent is a leeching and peering torrents tool built in Rust")
        .arg(
            Arg::new("torrent file(s)")
                .short('t')
                .long("torrent")
                .required(true)
                .help("Torrent file(s)")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("Pretty print file")
                .short('p')
                .long("pretty-print-file")
                .required(false)
                .help("Pretty print file(s) in JSON format")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("Dump peer(s)")
                .short('d')
                .long("dump-peers")
                .required(false)
                .help("Display peers ip and port returned by the tracker")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("Verbose")
                .short('v')
                .long("verbose")
                .required(false)
                .help("Display all the network communications with the peers")
                .action(ArgAction::Count),
        )
        .get_matches();

    let torrents = matches
        .get_many::<String>("torrent file(s)")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();

    let ppf = matches.get_count("Pretty print file");
    let dp = matches.get_count("Dump peer(s)");
    let log = matches.get_count("Verbose");

    for torrent_file in torrents {
        let mut info_string = String::from("");
        match encode_info_field(torrent_file) {
            Ok(string) => {
                info_string = string;
            }
            Err(e) => {
                println!("Failed to extract info field");
                std::process::exit(1);
            }
        }

        let contents = read_content(torrent_file).unwrap();

        match decode_bencoded_string(contents) {
            Ok(map) => {
                // Use the map here
                let meta_info = parsing::parse_metainfo(map.clone());
                if ppf == 1 {
                    println!("{{\n{}\n}}\n", meta_info);
                } else {
                    let mut hasher = Sha1::new();
                    hasher.update(info_string.as_bytes());
                    let info_hash: [u8; 20] = hasher
                        .finalize()
                        .try_into()
                        .expect("GenericArray<, 20> == [; 20]");

                    let info_hash_6_bytes = info_hash_to_string(&info_hash)[..6].to_string();

                    
                    if log == 1 {
                        println!(
                            "{}: tracker: requesting peers to {}",
                            info_hash_6_bytes, meta_info.announce
                        );
                    }
                    let tracker_reponse = send_request(map.clone(), info_hash).await;
                    if dp == 1 {
                        dump_peers(tracker_reponse.clone());
                    }

                    let peers = tracker_reponse.clone().peers.0;

                    for (i, peer_ip) in peers.iter().enumerate() {
                        let handshake_result = time::timeout(
                            Duration::from_secs(5),
                            handshake_peer(*peer_ip, info_hash),
                        )
                        .await;

                        match handshake_result {
                            Ok(peer) => {
                                if log == 1 {
                                    println!(
                                        "{}: peers: connect: {}: handshake",
                                        info_hash_6_bytes, peer_ip
                                    );
                                    println!(
                                        "{}: msg: send: {}: handshake",
                                        info_hash_6_bytes, peer_ip
                                    );
                                    println!(
                                        "{}: msg: recv: {}: handshake",
                                        info_hash_6_bytes, peer_ip
                                    );
                                }

                                let mut peer = tokio_util::codec::Framed::new(peer, MessageFrame);
                                let bitfield = peer
                                    .next()
                                    .await
                                    .expect("peer always sends a bitfields")
                                    .context("peer message was invalid")
                                    .unwrap();

                                if log == 1 {
                                    println!(
                                        "{}: msg: recv: {}: bitfield {:?}",
                                        info_hash_6_bytes, peer_ip, bitfield.payload
                                    );
                                }

                                peer.send(Message {
                                    tag: MessageTag::Interested,
                                    payload: Vec::new(),
                                })
                                .await
                                .context("send interested message")
                                .unwrap();
                                if log == 1 {
                                    println!(
                                        "{}: msg: send: {}: interested",
                                        info_hash_6_bytes, peer_ip
                                    );
                                }

                                let unchoke = peer
                                    .next()
                                    .await
                                    .expect("peer always sends an unchoke")
                                    .context("peer message was invalid")
                                    .unwrap();

                                if log == 1 {
                                    println!(
                                        "{}: msg: recv: {}: unchoke",
                                        info_hash_6_bytes, peer_ip
                                    );
                                }

                                let piece_i = 0;

                                println!("{}", meta_info.info.pieces.len());

                                let piece_hash = meta_info.info.pieces[piece_i];
                                let torrent_length = compute_length(&meta_info.info);
                                let piece_size = if piece_i == meta_info.info.pieces.len() - 1 {
                                    let md = torrent_length % meta_info.info.piece_length;
                                    if md == 0 {
                                        meta_info.info.piece_length
                                    } else {
                                        md
                                    }
                                } else {
                                    meta_info.info.piece_length
                                };

                                let nblocks = (piece_size + (BLOCK_MAX - 1)) / BLOCK_MAX;
                                println!("{nblocks}");
                                let mut all_blocks = Vec::with_capacity(piece_size);
                                for block in 0..nblocks {
                                    let block_size = if block == nblocks - 1 {
                                        let md = piece_size % BLOCK_MAX;
                                        if md == 0 {
                                            BLOCK_MAX
                                        } else {
                                            md
                                        }
                                    } else {
                                        BLOCK_MAX
                                    };
                                    let mut request = Request::new(
                                        piece_i as u32,
                                        (block * BLOCK_MAX) as u32,
                                        block_size as u32,
                                    );
                                    let request_bytes = Vec::from(request.as_bytes_mut());
                                    peer.send(Message {
                                        tag: MessageTag::Request,
                                        payload: request_bytes,
                                    })
                                    .await
                                    .with_context(|| format!("send request for block {block}"));

                                    if log == 1 {
                                        println!(
                                            "{}: msg: send: {}: request {} {} {}",
                                            info_hash_6_bytes,
                                            peer_ip,
                                            u32::from_ne_bytes(request.index),
                                            u32::from_ne_bytes(request.begin),
                                            u32::from_ne_bytes(request.length)
                                        );
                                    }

                                    let piece = peer
                                        .next()
                                        .await
                                        .expect("peer always sends a piece")
                                        .context("peer message was invalid")
                                        .unwrap();
                                    if log == 1 {
                                        println!(
                                            "{}: msg: recv: {}: piece",
                                            info_hash_6_bytes, peer_ip
                                        );
                                    }

                                    let piece = Piece::ref_from_bytes(&piece.payload[..])
                                        .expect("always get all Piece response fields from peer");
                                    assert_eq!(piece.index() as usize, piece_i);
                                    assert_eq!(piece.begin() as usize, block * BLOCK_MAX);
                                    assert_eq!(piece.block().len(), block_size);
                                    all_blocks.extend(piece.block());
                                }
                                assert_eq!(all_blocks.len(), piece_size);

                                if log == 1 {
                                    println!(
                                        "{}: peers: disconnect: {}",
                                        info_hash_6_bytes, peer_ip
                                    );
                                }

                                let output = PathBuf::from(&meta_info.info.name);
                                tokio::fs::write(&output, all_blocks)
                                    .await
                                    .context("write out downloaded piece")
                                    .unwrap();
                                println!("Piece {piece_i} downloaded to {}.", output.display());
                            }
                            Err(_) => {
                                //println!("Handshake timed out with peer: {}, trying next...", peer);
                            }
                        }
                    }
                    
                    //download::all(map, info_hash).await;
                }
            }

            Err(e) => {
                println!("Failed to decode: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

fn info_hash_to_string(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(2 * t.len());
    for &byte in t {
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}

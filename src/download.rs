use crate::bdecoder::OwnedValue;
use crate::parsing::parse_metainfo;
use crate::parsing::File;
use crate::parsing::MetaInfo;
use crate::peers::Peer;
use crate::peers::Piece;
use crate::piece::PieceFile;
use crate::tracker::compute_length;
use crate::tracker::send_request;
use crate::tracker::TrackerResponse;
use crate::BLOCK_MAX;
use anyhow::Context;
use futures_util::stream::StreamExt;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::collections::BinaryHeap;

pub(crate) async fn all(
    dict: BTreeMap<String, OwnedValue>,
    info_hash: [u8; 20],
) -> anyhow::Result<Downloaded> {
    let meta_info = parse_metainfo(dict.clone());
    let peer_info = send_request(dict, info_hash).await;

    let mut peer_list = Vec::new();
    let mut peers = futures_util::stream::iter(peer_info.peers.0.iter())
        .map(|&peer_addr| async move {
            let peer = Peer::new(peer_addr, info_hash).await;
            (peer_addr, peer)
        })
        .buffer_unordered(5);
    while let Some((peer_addr, peer)) = peers.next().await {
        match peer {
            Ok(peer) => {
                peer_list.push(peer);
                if peer_list.len() >= 5
                {
                    break;
                }
            }
            Err(e) => {
                println!("failed to connect to peer {peer_addr:?}: {e:?}");
            }
        }
    }
    
    drop(peers);
    let mut peers = peer_list;

    let mut need_pieces = BinaryHeap::new();
    let mut no_peers = Vec::new();
    for piece_i in 0..meta_info.info.pieces.len() {
        let piece = PieceFile::new(piece_i, &meta_info, &peers);
        if piece.peers().is_empty() {
            no_peers.push(piece);
        } else {
            need_pieces.push(piece);
        }
    }
    println!("len = {}", need_pieces.len());

    let length = compute_length(&meta_info.info);
    let mut all_pieces = vec![0; length];
    while let Some(piece) = need_pieces.pop() {
        let piece_size = piece.length();
        let nblocks = (piece_size + (BLOCK_MAX - 1)) / BLOCK_MAX;
        let peers: Vec<_> = peers
            .iter_mut()
            .enumerate()
            .filter_map(|(peer_i, peer)| piece.peers().contains(&peer_i).then_some(peer))
            .collect();

        let (submit, tasks) = kanal::bounded_async(nblocks);
        for block in 0..nblocks {
            submit
                .send(block)
                .await
                .expect("bound holds all these items");
        }
        let (finish, mut done) = tokio::sync::mpsc::channel(nblocks);
        let mut participants = futures_util::stream::futures_unordered::FuturesUnordered::new();
        for peer in peers {
            participants.push(peer.participate(
                piece.index(),
                piece_size,
                nblocks,
                submit.clone(),
                tasks.clone(),
                finish.clone(),
            ));
        }
        drop(submit);
        drop(finish);
        drop(tasks);

        println!("start receive loop");
        let mut all_blocks = vec![0u8; piece_size];
        let mut bytes_received = 0;
        loop {
            tokio::select! {
                joined = participants.next(), if !participants.is_empty() => {
                    // if a participant ends early, it's either slow or failed
                    println!("participant finished");
                    match joined {
                        None => {
                            // there are no peers!
                            // this must mean we are about to get None from done.recv(),
                            // so we'll handle it there
                            println!("No peer");
                        }
                        Some(Ok(_)) => {
                            // the peer gave up because it timed out
                            // nothing to do, except maybe de-prioritize this peer for later
                            println!("The peer is timed out");
                        }
                        Some(Err(e)) => {
                            // the peer failed and should be removed
                            // it already isn't participating in this piece any more, so this is
                            // more of an indicator that we shouldn't try this peer again, and
                            // should remove it from the global peer list
                            println!("A peer failed with an error: {:?}", e);
                        }
                    }
                }
                piece = done.recv() => {
                    if let Some(piece) = piece {
                        print!("got piece ");
                        // keep track of the bytes in message
                        let piece = Piece::ref_from_bytes(&piece.payload[..])
                            .expect("always get all Piece response fields from peer");
                        println!("{}", piece.index());
                        bytes_received += piece.block().len();
                        all_blocks[piece.begin() as usize..][..piece.block().len()].copy_from_slice(piece.block());
                        if bytes_received == piece_size {
                            // have received every piece
                            // this must mean that all participations have either exited or are
                            // waiting for more work -- in either case, it is okay to drop all the
                            // participant futures.
                            break;
                        }
                    } else {
                        println!("got pieces end");
                        // there are no peers left, so we can't progress!
                        break;
                    }
                }
            }
        }
        drop(participants);

        if bytes_received == piece_size {
            
        } else {
            anyhow::bail!("no peers left to get piece {}", piece.index());
        }

        let mut hasher = Sha1::new();
        hasher.update(&all_blocks);
        let hash: [u8; 20] = hasher
            .finalize()
            .try_into()
            .expect("GenericArray<_, 20> == [_; 20]");

        all_pieces[piece.index() * meta_info.info.piece_length..][..piece_size]
            .copy_from_slice(&all_blocks);
    }

    Ok(Downloaded {
        bytes: all_pieces,
        files: match &meta_info.info.length {
            length => vec![File {
                length: *length,
                md5sum: Some(String::from("")),
                path: meta_info.info.name.clone(),
            }],
            0 => vec![File::default()],
        },
    })
}

pub struct Downloaded {
    bytes: Vec<u8>,
    files: Vec<File>,
}

impl<'a> IntoIterator for &'a Downloaded {
    type Item = DownloadedFile<'a>;
    type IntoIter = DownloadedIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DownloadedIter::new(self)
    }
}

pub struct DownloadedIter<'d> {
    downloaded: &'d Downloaded,
    file_iter: std::slice::Iter<'d, File>,
    offset: usize,
}

impl<'d> DownloadedIter<'d> {
    fn new(d: &'d Downloaded) -> Self {
        Self {
            downloaded: d,
            file_iter: d.files.iter(),
            offset: 0,
        }
    }
}

impl<'d> Iterator for DownloadedIter<'d> {
    type Item = DownloadedFile<'d>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.file_iter.next()?;
        let bytes = &self.downloaded.bytes[self.offset..][..file.length];
        self.offset += file.length;
        Some(DownloadedFile { file, bytes })
    }
}

pub struct DownloadedFile<'d> {
    file: &'d File,
    bytes: &'d [u8],
}

impl<'d> DownloadedFile<'d> {

    pub fn bytes(&self) -> &'d [u8] {
        self.bytes
    }
}

use crate::tracker::compute_length;
use crate::{parsing::MetaInfo, peers::Peer};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq)]
pub struct PieceFile {
    peers: HashSet<usize>,
    piece_i: usize,
    length: usize,
    hash: [u8; 20],
}

impl Ord for PieceFile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.peers
            .len()
            .cmp(&other.peers.len())
            .then(self.peers.iter().cmp(other.peers.iter()))
            .then(self.hash.cmp(&other.hash))
            .then(self.length.cmp(&other.length))
            .then(self.piece_i.cmp(&other.piece_i))
    }
}

impl PartialOrd for PieceFile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PieceFile {
    pub(crate) fn new(piece_i: usize, meta_info: &MetaInfo, peers: &[Peer]) -> Self {
        let piece_hash = meta_info.info.pieces[piece_i];
        let piece_size = if piece_i == meta_info.info.pieces.len() - 1 {
            let torrent_length = compute_length(&meta_info.info);
            let md = torrent_length % meta_info.info.piece_length;
            if md == 0 {
                meta_info.info.piece_length
            } else {
                md
            }
        } else {
            meta_info.info.piece_length
        };

        let peers = peers
            .iter()
            .enumerate()
            .filter_map(|(peer_i, peer)| peer.has_piece(piece_i).then_some(peer_i))
            .collect();

        Self {
            peers,
            piece_i,
            length: piece_size,
            hash: piece_hash,
        }
    }

    pub(crate) fn peers(&self) -> &HashSet<usize> {
        &self.peers
    }

    pub(crate) fn index(&self) -> usize {
        self.piece_i
    }

    pub(crate) fn hash(&self) -> [u8; 20] {
        self.hash
    }

    pub(crate) fn length(&self) -> usize {
        self.length
    }
}

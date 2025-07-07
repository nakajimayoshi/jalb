use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use crate::peer::Peer;

pub trait Selector {
    fn select_peer(&mut self) -> Option<&Peer>;
    fn add_peer(&mut self, peer: Peer);
}

#[derive(Debug)]
pub struct RoundRobin {
    last_idx: usize,
    pool: Vec<Peer>,
}

impl RoundRobin {
    pub fn new() -> RoundRobin {
        Self {
            last_idx: 0,
            pool: Vec::new(),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        RoundRobin::new()
    }
}

impl Selector for RoundRobin {
    fn select_peer(&mut self) -> Option<&Peer> {
        if self.last_idx == self.pool.len() {
            self.last_idx = 0;
            return self.pool.get(self.last_idx);
        }

        self.last_idx += 1;
        self.pool.get(self.last_idx)
    }

    fn add_peer(&mut self, peer: Peer) {
        self.pool.push(peer)
    }
}

#[cfg(test)]
mod tests {
    use super::{RoundRobin, Selector};
    use crate::peer::Peer;

    #[test]
    fn test_round_robin() {
        let peers = vec![
            Peer::new("127.0.0.1:8080").unwrap(),
            Peer::new("127.0.0.1:8081").unwrap(),
            Peer::new("127.0.0.1:8082").unwrap(),
            Peer::new("127.0.0.1:8083").unwrap(),
            Peer::new("127.0.0.1:8084").unwrap(),
            Peer::new("127.0.0.1:8085").unwrap(),
        ];

        let mut selector = RoundRobin::default();

        for peer in peers {
            selector.add_peer(peer);
        }

        let peer1 = selector.select_peer();
    }
}

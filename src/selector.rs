use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use crate::peer::Peer;

pub trait Selector {
    fn select_peer(&mut self) -> Option<&Peer>;
    fn with_peers(&mut self, nodes: Vec<Peer>);
}

#[derive(Debug)]
pub struct RoundRobin {
    last_idx: usize,
    pool: HashMap<usize, Peer>,
}

impl RoundRobin {
    pub fn new() -> RoundRobin {
        Self {
            last_idx: 0,
            pool: HashMap::new(),
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
            return self.pool.get(&self.last_idx);
        }

        self.last_idx += 1;
        self.pool.get(&self.last_idx)
    }

    fn with_peers(&mut self, peers: Vec<Peer>) {
        for (i, node) in peers.into_iter().enumerate() {
            self.pool.insert(i, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RoundRobin, Selector};
    use crate::peer::Peer;

    #[test]
    fn test_round_robin() {
        let selector = RoundRobin::default();
    }
}

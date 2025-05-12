use crate::backend::Backend;

pub trait Selector {
    fn select_service(&mut self, backends: &Vec<Backend>) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub struct RoundRobinSelector {
    last_idx: usize,
}

impl RoundRobinSelector {
    pub fn new() -> RoundRobinSelector {
        Self { last_idx: 0 }
    }
}

impl Default for RoundRobinSelector {
    fn default() -> Self {
        RoundRobinSelector::new()
    }
}

impl Selector for RoundRobinSelector {
    fn select_service(&mut self, backends: &Vec<Backend>) -> usize {
        if backends.is_empty() {
            return 0;
        }

        let current = self.last_idx;

        self.last_idx = (self.last_idx + 1) % backends.len();

        current
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::Backend;

    use super::{RoundRobinSelector, Selector};

    #[test]
    fn should_select_next() {
        let mut selector = RoundRobinSelector::new();

        let backends = vec![
            Backend::new("127.0.0.1:3000").unwrap(),
            Backend::new("127.0.0.1:3002").unwrap(),
            Backend::new("127.0.0.1:3003").unwrap(),
            Backend::new("127.0.0.1:3004").unwrap(),
        ];

        assert_eq!(selector.select_service(&backends), 0);
        assert_eq!(selector.select_service(&backends), 1);
        assert_eq!(selector.select_service(&backends), 2);
        assert_eq!(selector.select_service(&backends), 3);

        assert_eq!(selector.select_service(&backends), 0);
    }
}

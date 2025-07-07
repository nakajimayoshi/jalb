use crate::{config::JalbConfig, security::Security, selector::Selector};

pub struct NetworkLoadBalancer<T: Selector> {
    security: Security,
    selector: T,
}

impl<T: Selector + Default> NetworkLoadBalancer<T> {
    pub fn new(config: &JalbConfig) -> Self {
        Self {
            security: config.security(),
            selector: T::default(),
        }
    }
}

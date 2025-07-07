use crate::{
    backend::Backend,
    config::{BackendOptions, Config},
    security::Security,
    selector::Selector,
};

pub struct NetworkLoadBalancer<T: Selector> {
    security: Security,
    selector: T,
    backend: Backend,
}

impl<T: Selector + Default> NetworkLoadBalancer<T> {
    pub(crate) fn new_from_config(
        backend_config: &BackendOptions,
        security_config: Security,
    ) -> Self {
        let backend = Backend::from_config(backend_config);
        Self {
            security: security_config,
            selector: T::default(),
            backend: backend,
        }
    }
}

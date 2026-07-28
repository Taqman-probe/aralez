use std::collections::HashMap;
use std::sync::LazyLock;
use pingora_proxy::Session;
use crate::utils::structs::InnerAuth;
use aralez_spec::{AuthPluginEntry, AuthFactory};

// Scans the inventory at startup and stores the factories for all registered authentication plugins
static AUTH_FACTORIES: LazyLock<HashMap<&'static str, AuthFactory>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for entry in inventory::iter::<AuthPluginEntry> {
        map.insert(entry.name, entry.create);
    }
    map
});

pub async fn authenticate(auth: &InnerAuth, session: &mut Session) -> bool {
    if let Some(factory) = AUTH_FACTORIES.get(&*auth.auth_type) {
        let validator = factory(auth.auth_cred.clone());
        validator.validate(session).await
    } else {
        log::warn!("Unsupported authentication mechanism: {}", &*auth.auth_type);
        false
    }
}

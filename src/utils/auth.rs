use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use aralez_spec::{AuthPluginEntry, AuthFactory, AuthValidator};

// Scans the inventory at startup and stores the factories for all registered authentication plugins
static AUTH_FACTORIES: LazyLock<HashMap<&'static str, AuthFactory>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for entry in inventory::iter::<AuthPluginEntry> {
        map.insert(entry.name, entry.create);
    }
    map
});

pub fn create_validator(auth_type: &str, data: Option<noyalib::Value>)
-> Result<Arc<dyn AuthValidator>, Box<dyn std::error::Error>> {
    if let Some(factory) = AUTH_FACTORIES.get(auth_type) {
        factory(data)
    } else {
        Err(format!("Unsupported authentication mechanism: {}", auth_type).into())
    }
}

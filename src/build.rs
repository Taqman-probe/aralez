fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let res = winresource::WindowsResource::new();
        res.compile().unwrap();
    }

    let features: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_").map(|s| s.to_ascii_lowercase())
        })
        .collect();

    println!("cargo:rustc-env=ENABLED_FEATURES={}", features.join(","));
}
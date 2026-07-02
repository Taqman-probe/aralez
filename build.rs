fn main() {
    let features: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_").map(|s| s.to_ascii_lowercase())
        })
        .collect();

    println!("cargo:rustc-env=ENABLED_FEATURES={}", features.join(","));
}

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest.parent().expect("tests-compatibility is under workspace root");
    let cache_sources = workspace.join("target/compatibility/sources");

    if !cache_sources.exists() {
        println!(
            "cargo::warning=cargo run -p tests-compatibility -- prepare --preset core --preset acme"
        );
    }
}

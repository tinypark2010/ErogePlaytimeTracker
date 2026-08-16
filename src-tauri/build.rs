fn main() {
    let notices = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("THIRD_PARTY_LICENSES.txt");
    if !notices.exists() {
        let placeholder = "Eroge Playtime Tracker - Third-Party Licenses\n\nRun `npm run licenses` before packaging to generate the complete dependency notices.\n";
        std::fs::write(&notices, placeholder)
            .unwrap_or_else(|error| panic!("failed to create {}: {error}", notices.display()));
    }
    tauri_build::build()
}

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Ensure secrets.toml exists for include_str! / compile.
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let secrets = manifest.join("secrets.toml");
    let example = manifest.join("secrets.example.toml");
    if !secrets.exists() {
        if example.exists() {
            let _ = fs::copy(&example, &secrets);
        } else {
            fs::write(
                &secrets,
                "gemini_api_key = \"\"\ngemini_model = \"gemini-flash-latest\"\npresage_api_key = \"\"\ngoogle_client_id = \"\"\ngoogle_client_secret = \"\"\n",
            )
            .expect("write secrets.toml");
        }
    }
    println!("cargo:rerun-if-changed=secrets.toml");
    println!("cargo:rerun-if-changed=secrets.example.toml");
    tauri_build::build()
}

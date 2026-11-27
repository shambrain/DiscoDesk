//! Build script for the Studio.
//!
//! `tauri-build` embeds the frontend assets listed by `frontendDist` at compile
//! time, and fails if that directory does not exist. `cargo build` alone
//! therefore cannot build this crate from a clean checkout before the UI is
//! built.
//!
//! That ordering is unavoidable, but a bare Tauri error is confusing, so this
//! script checks first and fails with instructions. It does **not** create a
//! placeholder directory: a Studio built against a fake frontend would look
//! successful while serving nothing.

use std::path::Path;

fn main() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo");
    let dist = Path::new(&manifest_dir).join("../dist");

    if !dist.join("index.html").is_file() {
        panic!(
            "\n\nDiscoDesk Studio: the compiled frontend is missing.\n\n\
             Expected: {}\n\n\
             Build it first:\n\
             \x20   cd apps/discodesk-studio\n\
             \x20   npm ci\n\
             \x20   npm run build\n\n\
             `cargo` cannot produce this directory itself, and this script will not\n\
             fabricate a placeholder, because a Studio bundled with a fake frontend\n\
             would appear to work while serving nothing.\n",
            dist.display()
        );
    }

    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=capabilities");
    println!("cargo:rerun-if-changed=../dist/index.html");

    tauri_build::build();
}

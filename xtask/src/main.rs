//! `cargo xtask bundle [--release]`: builds Kite and assembles a self-contained
//! directory (`target/bundle`) with the binary, `libcef.so`, its resources and locales.

use std::{path::Path, process::ExitCode};

const BINARY: &str = "kite";
const OUTPUT_DIR: &str = "target/bundle";
const USAGE: &str = "usage: cargo xtask bundle [--release]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("bundle") => {
            let release = args.iter().any(|arg| arg == "--release");
            bundle(Path::new(OUTPUT_DIR), release)
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "linux")]
fn bundle(output: &Path, release: bool) -> ExitCode {
    match cef::build_util::linux::build_bundle(output, BINARY, release) {
        Ok(path) => {
            println!("Bundle ready. Run: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("bundle failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn bundle(_output: &Path, _release: bool) -> ExitCode {
    eprintln!("`cargo xtask bundle` is only implemented for Linux.");
    ExitCode::FAILURE
}

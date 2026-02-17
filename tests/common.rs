//! Common utilities for integration tests.
//!
//! This module provides shared functionality for integration tests that depend
//! on example binaries. It ensures examples are built before tests run and
//! provides convenient functions to access example binary paths.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, ensure};

/// Build all example binaries if they haven't been built yet.
///
/// Checks if the example binaries exist in the target directory and builds them
/// if necessary. This is safe to call multiple times - it will only build once.
pub fn ensure_examples_built() -> Result<()> {
    let target_dir = get_target_dir();
    let profile = get_profile();
    let examples_dir = target_dir.join(profile).join("examples");

    let examples = get_example_names()?;

    let all_exist = examples.iter().all(|name| examples_dir.join(name).exists());

    if !all_exist {
        eprintln!("Example binaries not found, building...");

        let status = Command::new("cargo")
            .args(["build", "--examples"])
            .status()?;

        ensure!(
            status.success(),
            "cargo build --examples failed with status: {}",
            status
        );

        eprintln!("Examples built successfully");
    }

    Ok(())
}

/// Get the path to a specific example binary.
///
/// Ensures examples are built before returning the path.
///
/// ```no_run
/// use common::get_example_path;
///
/// let path = get_example_path("basic_server").unwrap();
/// // path will be something like: /path/to/project/target/debug/examples/basic_server
/// ```
pub fn get_example_path(name: &str) -> Result<PathBuf> {
    ensure_examples_built()?;

    let target_dir = get_target_dir();
    let profile = get_profile();
    let binary_path = target_dir.join(profile).join("examples").join(name);

    ensure!(
        binary_path.exists(),
        "Example binary '{}' not found at: {}",
        name,
        binary_path.display()
    );

    Ok(binary_path)
}

/// Get the target directory path from the environment.
///
/// Uses the `CARGO_MANIFEST_DIR` environment variable which is set by cargo
/// during test execution.
fn get_target_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir).join("target")
}

/// Get the current build profile (debug or release).
///
/// Uses the `PROFILE` environment variable set by cargo. Defaults to "debug"
/// if not set.
fn get_profile() -> String {
    std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string())
}

/// Discover all example binary names from the examples directory.
///
/// Reads the `examples/` directory in the source code and returns the
/// names of all `.rs` files (without extension), which correspond to the
/// binary names.
fn get_example_names() -> Result<Vec<String>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_source_dir = PathBuf::from(manifest_dir).join("examples");

    let mut example_names = Vec::new();

    for entry in fs::read_dir(examples_source_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "rs") {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|name| example_names.push(name.to_string()));
        }
    }

    ensure!(
        !example_names.is_empty(),
        "No example files found in examples/ directory"
    );

    Ok(example_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_dir_ends_with_target() {
        let dir = get_target_dir();
        assert!(dir.ends_with("target"));
    }

    #[test]
    fn profile_is_valid() {
        let profile = get_profile();
        assert!(profile == "debug" || profile == "release" || profile == "test");
    }
}

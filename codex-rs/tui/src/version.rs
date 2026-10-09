/// The current Codex CLI version as embedded at compile time.
#[cfg(not(test))]
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable UI fixture version across Cargo and Bazel test builds.
#[cfg(test)]
pub const CODEX_CLI_VERSION: &str = "0.0.0";

//! Structured error type for SkillPack internals.
//!
//! Commands continue to return `Result<T, String>` to preserve the existing
//! frontend contract, but internal logic can build a `SkillError` and let the
//! `Display` implementation produce a user-friendly message via
//! `.map_err(SkillError::to_string)` (or the `err_string()` helper).

use std::io;

/// Categorised error type. Kept deliberately small; each variant carries the
/// contextual information useful for both logging and user-facing messages.
///
/// Currently used as the structured foundation for future per-command error
/// reporting; existing commands still return `Result<T, String>` and convert
/// via `Display`. Suppress dead-code warnings until command sites adopt it.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("{0} not found")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArg(String),

    #[error("Unknown toolchain: {0}. Supported: codex, agents, claude, gemini, cursor")]
    Toolchain(String),

    #[error("{0}")]
    Other(String),
}

impl SkillError {
    /// Convenience constructor for an ad-hoc message.
    #[allow(dead_code)]
    pub fn msg<S: Into<String>>(s: S) -> Self {
        SkillError::Other(s.into())
    }
}

/// Convert any error into the string form expected by Tauri command signatures.
#[allow(dead_code)]
pub fn err_string<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

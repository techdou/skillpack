//! End-to-end validation against the user's REAL `~/.codex/config.toml`.
//!
//! These tests are `#[ignore]` by default because they depend on a live Codex
//! install and would be flaky/non-reproducible in CI. Run them manually on a
//! machine that has Codex configured:
//!
//! ```sh
//! cargo test --test codex_real_config -- --ignored --nocapture
//! ```
//!
//! They exercise the production read path (`codex::config::*`) against genuine
//! data, catching the kind of schema drift that synthetic fixtures cannot —
//! e.g. a plugin key format, an inline-table env, or a missing `[features]`
//! entry that the docs did not mention.

use skillpack_core::codex;

/// The real config path exists and parses under the lock.
#[test]
#[ignore]
fn real_config_resolves_and_parses() {
    let path = codex::config::resolve_config_path().expect("config.toml should resolve");
    assert!(path.exists(), "{} should exist", path.display());
    // Reading under the lock must not panic or error.
    let _ = codex::config::with_codex_config(|doc| doc.to_string()).expect("read under lock");
}

/// Every plugin entry has a non-empty key, name, and a parsable enabled bool.
/// The cache manifest merge must not blow up even when a plugin dir is missing.
#[test]
#[ignore]
fn real_plugin_list_well_formed() {
    let plugins = codex::config::with_codex_config(codex::config::list_plugins)
        .expect("list_plugins on real config");
    eprintln!("found {} plugins", plugins.len());
    assert!(!plugins.is_empty(), "expected some plugins on a live install");
    for p in &plugins {
        assert!(!p.key.is_empty(), "plugin key empty: {:?}", p);
        assert!(!p.name.is_empty(), "plugin name empty: {:?}", p);
        // enabled is always a definite bool after our normalisation.
    }
}

/// `[features]plugins` reads without error. On this install it is absent, so
/// the default-true path must be exercised (this is the B4 fix).
#[test]
#[ignore]
fn real_features_plugins_reads_without_error() {
    let enabled = codex::config::with_codex_config(codex::config::features_plugins_enabled)
        .expect("features read");
    eprintln!("features.plugins enabled = {}", enabled);
}

/// Marketplaces parse from `[marketplaces.*]`.
#[test]
#[ignore]
fn real_marketplace_list_well_formed() {
    let mks = codex::config::with_codex_config(codex::config::list_marketplaces)
        .expect("list_marketplaces on real config");
    eprintln!("found {} marketplaces", mks.len());
    for m in &mks {
        assert!(!m.name.is_empty(), "marketplace name empty: {:?}", m);
    }
}

/// Top-level MCP servers parse, including inline-table env (the B-adjacent fix
/// for `as_table_like`).
#[test]
#[ignore]
fn real_mcp_list_well_formed() {
    let servers = codex::config::with_codex_config(codex::config::list_mcp_servers)
        .expect("list_mcp_servers on real config");
    eprintln!("found {} mcp servers", servers.len());
    assert!(!servers.is_empty(), "expected some MCP servers");
    for s in &servers {
        assert!(!s.name.is_empty(), "mcp name empty: {:?}", s);
        // Every server must define either a command or a url.
        assert!(
            s.command.is_some() || s.url.is_some(),
            "mcp {} has neither command nor url: {:?}",
            s.name,
            s
        );
    }
}

/// CLI resolution behaves correctly. This is an environment-dependent check:
/// `codex` is only resolvable when it is on PATH or `CODEX_CLI_PATH` is set.
/// We assert the documented behaviour rather than a specific install location:
///   - if `CODEX_CLI_PATH` is set, it must be honoured verbatim;
///   - otherwise resolution may legitimately fail (codex not on PATH), which
///     we report as a diagnostic, not a failure.
#[test]
#[ignore]
fn real_cli_resolves_or_reports_absence() {
    if let Ok(explicit) = std::env::var("CODEX_CLI_PATH") {
        if !explicit.is_empty() {
            let resolved = codex::cli::resolve_codex_cli()
                .expect("CODEX_CLI_PATH set but resolve failed");
            assert_eq!(resolved, explicit, "env override must be honoured");
            eprintln!("codex CLI (via CODEX_CLI_PATH): {}", resolved);
            return;
        }
    }
    match codex::cli::resolve_codex_cli() {
        Ok(path) => eprintln!("codex CLI (via PATH): {}", path),
        Err(e) => eprintln!(
            "codex not on PATH (ok on dev machines without it): {} \n\
             set CODEX_CLI_PATH to point at codex.exe to exercise install/uninstall commands",
            e
        ),
    }
}

use std::{collections::HashMap, path::PathBuf};

use anyhow::{bail, Context, Result};
use flags2env::BundledFlags2Env;

const CONFIG_NAME: &str = ".cli-flags.toml";

/// Resolve and apply the canonical flags2env schema exactly once, before Qt or
/// any worker thread starts. Precedence is CLI > environment > `.env` > schema
/// default, and secret-bearing keys are intentionally not declared as flags.
pub fn init() -> Result<()> {
    let config_path = find_config()?;
    let parser = BundledFlags2Env::new();
    parser
        .audit_config(config_path.to_str())
        .map_err(|_| anyhow::anyhow!("flags2env rejected the desktop flag schema"))?;
    let argv = std::env::args().collect::<Vec<_>>();
    let values = parse(&parser, &argv, &config_path)?;

    // This is the single-threaded process entry point. All environment writes
    // happen before Qt, Bluetooth, reminder, or network workers are created.
    for (key, value) in values {
        std::env::set_var(key, value);
    }
    Ok(())
}

fn parse(
    parser: &BundledFlags2Env,
    argv: &[String],
    config_path: &std::path::Path,
) -> Result<HashMap<String, String>> {
    let parsed = parser
        .parse_structured(argv, config_path.to_str())
        .map_err(|_| anyhow::anyhow!("flags2env could not parse the desktop configuration"))?;
    if !parsed.unknown_options.is_empty() || !parsed.errors.is_empty() {
        bail!(
            "flags2env rejected {} unknown option(s) and {} invalid value(s)",
            parsed.unknown_options.len(),
            parsed.errors.len()
        );
    }
    Ok(parsed.flags)
}

fn find_config() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("FLAGS2ENV_CONFIG") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Ok(path);
        }
        bail!("FLAGS2ENV_CONFIG does not name a readable file");
    }

    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join(CONFIG_NAME));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(CONFIG_NAME));
            candidates.push(directory.join("../Resources").join(CONFIG_NAME));
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CONFIG_NAME));
    candidates.into_iter().find(|candidate| candidate.is_file()).context(
        ".cli-flags.toml was not found beside the app, in Resources, or in the working directory",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn canonical_schema_executes_through_flags2env() {
        let parser = BundledFlags2Env::new();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CONFIG_NAME);
        parser.audit_config(path.to_str()).unwrap();
        let values = parse(
            &parser,
            &strings(&[
                "happy-wakey",
                "--platform-url",
                "https://platform.example.test",
                "--config-dir=/tmp/happy-wakey-test",
            ]),
            &path,
        )
        .unwrap();
        assert_eq!(
            values.get("HAPPY_WAKEY_PLATFORM_URL").map(String::as_str),
            Some("https://platform.example.test")
        );
        assert_eq!(
            values.get("CONFIG_DIR").map(String::as_str),
            Some("/tmp/happy-wakey-test")
        );
    }

    #[test]
    fn secret_shaped_cli_options_fail_without_echoing_values() {
        let parser = BundledFlags2Env::new();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CONFIG_NAME);
        let error = parse(
            &parser,
            &strings(&["happy-wakey", "--newsapi-key=do-not-print-this"]),
            &path,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("1 unknown option"));
        assert!(!error.contains("do-not-print-this"));
    }
}

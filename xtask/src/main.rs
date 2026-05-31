#![allow(clippy::module_name_repetitions)]

use std::process::Command;

use anyhow::{Context, bail};

fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_else(|| "ci".to_owned());
    match task.as_str() {
        "ci" => ci(),
        "loc" => run("bash", &["scripts/check-loc.sh", "."]),
        "forbid-rpc" => run("bash", &["scripts/forbid-stock-pi-rpc.sh", "."]),
        "sync-protocol" => run("bash", &["scripts/sync-protocol.sh", "--write"]),
        "check-protocol" => run("bash", &["scripts/sync-protocol.sh", "--check"]),
        other => bail!("unknown xtask command: {other}"),
    }
}

fn ci() -> anyhow::Result<()> {
    run("cargo", &["fmt", "--all", "--check"])?;
    run("cargo", &["test", "-p", "pi-bridge-types"])?;
    run("bash", &["scripts/sync-protocol.sh", "--check"])?;
    run(
        "cargo",
        &["check", "--workspace", "--all-targets", "--all-features"],
    )?;
    run(
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--all-features"],
    )?;
    run("npm", &["--prefix", "node", "run", "typecheck"])?;
    run("npm", &["--prefix", "node", "run", "build"])?;
    run("cargo", &["test", "--workspace", "--all-features"])?;
    run("npm", &["--prefix", "node", "test"])?;
    run("npm", &["--prefix", "node", "run", "check-protocol"])?;
    run("bash", &["scripts/check-loc.sh", "."])?;
    run("bash", &["scripts/forbid-stock-pi-rpc.sh", "."])
}

fn run(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to start {program}"))?;
    if !status.success() {
        bail!("command failed: {program} {}", args.join(" "));
    }
    Ok(())
}

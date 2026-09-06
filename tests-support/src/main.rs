use std::path::PathBuf;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        bail!("usage: tests-support <prepare|lock> ...");
    };
    if command == "prepare" {
        let Some(lock_path) = arguments.next().map(PathBuf::from) else {
            bail!("missing lock path");
        };
        let Some(cache_root) = arguments.next().map(PathBuf::from) else {
            bail!("missing cache root");
        };
        if arguments.next().is_some() {
            bail!("usage: tests-support prepare <lock-path> <cache-root>");
        }
        let sources = tests_support::prepare(&lock_path, &cache_root)?;
        println!("Prepared {} registry packages.", sources.len());
    } else if command == "lock" {
        let Some(package_set_version) = arguments.next().and_then(|value| value.into_string().ok())
        else {
            bail!("missing or invalid package set version");
        };
        let Some(lock_path) = arguments.next().map(PathBuf::from) else {
            bail!("missing lock path");
        };
        let roots = arguments
            .map(|value| value.into_string())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| anyhow::anyhow!("invalid root package name"))?;
        tests_support::write_lock(&package_set_version, lock_path, roots)?;
    } else {
        bail!("unknown command {:?}; expected `prepare` or `lock`", command);
    }
    Ok(())
}

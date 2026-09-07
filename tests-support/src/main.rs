use std::path::PathBuf;

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        bail!("usage: tests-support prepare <configuration-path> <cache-root>");
    };
    if command == "prepare" {
        let Some(configuration_path) = arguments.next().map(PathBuf::from) else {
            bail!("missing configuration path");
        };
        let Some(cache_root) = arguments.next().map(PathBuf::from) else {
            bail!("missing cache root");
        };
        if arguments.next().is_some() {
            bail!("usage: tests-support prepare <configuration-path> <cache-root>");
        }
        let sources = tests_support::prepare(&configuration_path, &cache_root)?;
        println!("Prepared {} registry packages.", sources.len());
    } else {
        bail!("unknown command {:?}; expected `prepare`", command);
    }
    Ok(())
}

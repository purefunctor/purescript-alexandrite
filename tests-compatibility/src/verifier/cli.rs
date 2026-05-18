use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use tests_compatibility::Preset;

pub const DEFAULT_REGISTRY_DIR: &str = "target/compatibility/registry";
pub const DEFAULT_INDEX_DIR: &str = "target/compatibility/registry-index";

#[derive(Debug, Parser)]
#[command(name = "tests-compatibility")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Prepare(PrepareArgs),
    Verify(VerifyArgs),
}

#[derive(Debug, Args)]
pub struct PrepareArgs {
    #[arg(long, default_value = DEFAULT_REGISTRY_DIR)]
    pub registry_dir: PathBuf,
    #[arg(long, default_value = DEFAULT_INDEX_DIR)]
    pub index_dir: PathBuf,
    #[arg(long)]
    pub package_set: Option<String>,
    #[arg(long = "preset", value_enum)]
    pub presets: Vec<Preset>,
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[arg(long, default_value = DEFAULT_REGISTRY_DIR)]
    pub registry_dir: PathBuf,
    #[arg(long, default_value = DEFAULT_INDEX_DIR)]
    pub index_dir: PathBuf,
    #[arg(long)]
    pub package_set: Option<String>,
    #[arg(long = "package", value_parser = parse_package_name)]
    pub packages: Vec<String>,
    #[arg(long = "preset", value_enum)]
    pub presets: Vec<Preset>,
    #[arg(long)]
    pub json_output: Option<PathBuf>,
    #[arg(long)]
    pub cache_dir: Option<PathBuf>,
}

fn parse_package_name(value: &str) -> Result<String, String> {
    if let Some(stripped) = value.strip_prefix("purescript-") {
        return Err(format!("use registry package name '{stripped}' instead of '{value}'"));
    }

    Ok(value.to_string())
}

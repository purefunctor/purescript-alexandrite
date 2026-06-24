use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};
use tracing::level_filters::LevelFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(about, version(VERSION))]
pub struct Config {
    #[arg(long)]
    pub stdio: bool,
    /// Print log path.
    #[arg(long)]
    pub log_file: bool,
    /// Log level for the query engine.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub query_log: LevelFilter,
    /// Log level for the language server.
    #[arg(long, value_name("LevelFilter"), default_value("info"))]
    pub lsp_log: LevelFilter,
    /// Log level for the type checker.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub checking_log: LevelFilter,
    /// Command to use to get source files.
    ///
    /// This argument also disables the spago.lock integration.
    #[arg(long)]
    pub source_command: Option<String>,

    /// Publish diagnostics on textDocument/didOpen.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_open: bool,

    /// Publish diagnostics on textDocument/didSave.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_save: bool,

    /// Publish diagnostics on textDocument/didChange (opt-in).
    #[arg(long, default_value_t = false)]
    pub diagnostics_on_change: bool,
}

#[derive(Debug, Parser)]
#[command(about, version(VERSION))]
pub struct Cli {
    /// Print log path.
    #[arg(long)]
    pub log_file: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the language server.
    Lsp(LspOptions),
    /// Generate documentation.
    Docs(DocsOptions),
}

#[derive(Debug, Args)]
pub struct LoggingOptions {
    /// Log level for the query engine.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub query_log: LevelFilter,

    /// Log level for the type checker.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub checking_log: LevelFilter,
}

#[derive(Debug, Args)]
pub struct LspOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,

    #[arg(long)]
    pub stdio: bool,

    /// Log level for the language server.
    #[arg(long, value_name("LevelFilter"), default_value("info"))]
    pub lsp_log: LevelFilter,

    /// Command to use to get source files.
    ///
    /// This argument also disables the spago.lock integration.
    #[arg(long)]
    pub source_command: Option<String>,

    /// Publish diagnostics on textDocument/didOpen.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_open: bool,

    /// Publish diagnostics on textDocument/didSave.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_save: bool,

    /// Publish diagnostics on textDocument/didChange.
    #[arg(long, default_value_t = false)]
    pub diagnostics_on_change: bool,
}

#[derive(Debug, Args)]
pub struct DocsOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,
    /// Output directory for the generated documentation.
    #[arg(long, value_name("DIR"), default_value("docs"))]
    pub output_folder: PathBuf,
}

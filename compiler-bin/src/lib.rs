use std::process;

use clap::Parser;

pub mod cli;
pub mod logging;
pub mod lsp;

pub fn run() {
    let cli = cli::Cli::parse();

    if cli.log_file {
        eprintln!("Log file: {:?}", logging::temporary_log_file());
    }

    let command = cli.command.unwrap_or_else(|| {
        let options = cli::LspOptions::default();
        cli::Command::Lsp(options)
    });

    match command {
        cli::Command::Lsp(options) => {
            logging::start(logging::LoggingFilters {
                query_log: options.logging.query_log,
                checking_log: options.logging.checking_log,
                lsp_log: options.lsp_log,
            });
            lsp::start(lsp::LspConfig {
                source_command: options.source_command,
                diagnostics_on_open: options.diagnostics_on_open,
                diagnostics_on_save: options.diagnostics_on_save,
                diagnostics_on_change: options.diagnostics_on_change,
            });
        }
        cli::Command::Docs(_) => {
            eprintln!("`alexandrite docs` is not implemented yet.");
            process::exit(1);
        }
    }
}

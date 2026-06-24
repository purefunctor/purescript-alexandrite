pub mod error;

use std::path::PathBuf;

use crate::cli::PackageSpecs;

pub struct DocsConfig {
    pub output: PathBuf,
    pub packages: PackageSpecs,
}

pub fn start(_config: DocsConfig) {}

// alexandrite docs --package effect@v0.0.0 src/**/*.purs

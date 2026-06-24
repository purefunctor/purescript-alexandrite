use std::path::PathBuf;

pub struct DocsConfig {
    pub output_folder: PathBuf,
}

pub fn start(_config: DocsConfig) {}

// alexandrite docs --package effect@v0.0.0 src/**/*.purs

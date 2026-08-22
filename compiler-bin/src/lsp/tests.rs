use std::{fs, io};

use building::lifecycle::DiskObservation;
use lsp_types::Url;
use tempfile::tempdir;

use super::{observe_disk, source_unit_from_foreign_uri, source_unit_from_source_uri};

#[test]
fn source_and_foreign_uris_produce_the_same_unit_key() {
    let directory = tempdir().unwrap();
    let source_path = directory.path().join("Source Files").join("Main.purs");
    let foreign_path = source_path.with_extension("js");
    let source_uri = Url::from_file_path(source_path).unwrap();
    let foreign_uri = Url::from_file_path(foreign_path).unwrap();

    let from_source = source_unit_from_source_uri(&source_uri).unwrap();
    let from_foreign = source_unit_from_foreign_uri(&foreign_uri).unwrap();

    assert_eq!(from_source, from_foreign);
    assert_eq!(from_source.source(), source_uri.as_str());
    assert_eq!(from_source.foreign(), foreign_uri.as_str());
}

#[test]
fn disk_observation_distinguishes_content_absence_and_invalid_locators() {
    let directory = tempdir().unwrap();
    let source_path = directory.path().join("Main.purs");
    let source_uri = Url::from_file_path(&source_path).unwrap();

    fs::write(&source_path, "module Main where\n").unwrap();
    assert!(matches!(
        observe_disk(&source_uri),
        DiskObservation::Found(content) if content.as_ref() == "module Main where\n"
    ));

    fs::remove_file(source_path).unwrap();
    assert_eq!(observe_disk(&source_uri), DiskObservation::NotFound);

    let invalid_uri = Url::parse("untitled:Main.purs").unwrap();
    assert!(matches!(
        observe_disk(&invalid_uri),
        DiskObservation::Failed(failure) if failure.kind() == io::ErrorKind::InvalidInput
    ));
}

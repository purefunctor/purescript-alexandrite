//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use serde::Deserialize;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Deserialize)]
struct CheckResult {
    terms: Vec<String>,
    types: Vec<String>,
    synonyms: Vec<SynonymExpansion>,
    errors: Vec<CheckError>,
    timing: CheckTiming,
}

#[derive(Deserialize)]
struct SynonymExpansion {
    name: String,
    expansion: String,
    quantified_variables: u32,
    kind_variables: u32,
    type_variables: u32,
}

#[derive(Deserialize)]
struct CheckError {
    kind: String,
    message: String,
    location: Option<String>,
}

#[derive(Deserialize)]
struct CheckTiming {
    lex: f64,
    layout: f64,
    parse: f64,
    stabilize: f64,
    index: f64,
    resolve: f64,
    lower: f64,
    check: f64,
    total: f64,
}

impl CheckTiming {
    fn all(&self) -> [f64; 9] {
        [
            self.lex,
            self.layout,
            self.parse,
            self.stabilize,
            self.index,
            self.resolve,
            self.lower,
            self.check,
            self.total,
        ]
    }
}

fn check(source: &str) -> CheckResult {
    serde_wasm_bindgen::from_value(docs_lib::check(source)).unwrap()
}

#[wasm_bindgen_test]
fn parse_returns_the_syntax_tree_and_timings() {
    let result = docs_lib::parse("module Main where\n\nvalue = 42");
    let empty_module = docs_lib::parse("module Main where");

    assert!(result.output().contains("Module"));
    assert_ne!(result.output(), empty_module.output());
    assert!(result.lex >= 0.0);
    assert!(result.layout >= 0.0);
    assert!(result.parse >= 0.0);
}

#[wasm_bindgen_test]
fn check_uses_registered_packages_until_they_are_cleared() {
    docs_lib::clear_packages();
    let library = "module Library where\n\ntype Identity a = a\n\nidentity :: forall a. Identity a -> a\nidentity value = value";
    assert_eq!(
        docs_lib::register_source("library/src/Library.purs", library).as_deref(),
        Some("Library")
    );

    let source = "module Main where\n\nimport Library (identity)\n\nanswer = identity 42";
    let checked = check(source);
    assert_eq!(checked.terms, ["answer :: Int"]);
    assert!(checked.types.is_empty());
    assert!(checked.synonyms.is_empty());
    assert!(checked.errors.is_empty());
    assert!(checked.timing.all().into_iter().all(|timing| timing >= 0.0));

    let synonym_source =
        "module Main where\n\ntype Identity a = a\n\nidentity :: forall a. Identity a -> a\nidentity value = value";
    let checked = check(synonym_source);
    assert!(checked.terms.iter().any(|term| term.starts_with("identity ::")));
    assert_eq!(checked.synonyms.len(), 1);
    assert_eq!(checked.synonyms[0].name, "Identity");
    assert!(checked.synonyms[0].expansion.starts_with("forall a."));
    assert!(checked.synonyms[0].expansion.contains("a"));
    assert_eq!(checked.synonyms[0].quantified_variables, 1);
    assert_eq!(checked.synonyms[0].kind_variables, 0);
    assert_eq!(checked.synonyms[0].type_variables, 1);
    assert!(checked.errors.is_empty());

    docs_lib::clear_packages();
    let checked = check(source);
    let error = checked.errors.iter().find(|error| error.kind == "NotInScope").unwrap();
    assert!(!error.message.is_empty());
    assert!(error.location.as_deref().is_some_and(|location| location.contains("..")));
}

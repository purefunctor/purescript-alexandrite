use std::iter;
use std::sync::Arc;

use rustc_hash::FxHashSet;
use smol_str::SmolStr;

#[derive(Debug, Default)]
pub(super) struct NameAllocator {
    reserved: Arc<FxHashSet<SmolStr>>,
    names: FxHashSet<SmolStr>,
}

impl NameAllocator {
    pub(super) fn with_reserved(reserved: Arc<FxHashSet<SmolStr>>) -> NameAllocator {
        NameAllocator { reserved, names: FxHashSet::default() }
    }

    pub(super) fn allocate(&mut self, preferred: &str) -> String {
        let mut normalized = normalize_identifier(preferred);
        if identifier_is_reserved(&normalized) {
            normalized.insert(0, '$');
        }
        let mut candidate = normalized.clone();
        let mut suffix = 1;
        while self.reserved.contains(candidate.as_str())
            || !self.names.insert(SmolStr::new(&candidate))
        {
            candidate = format!("{normalized}${suffix}");
            suffix += 1;
        }
        candidate
    }

    pub(super) fn allocated_names(&self) -> impl Iterator<Item = &SmolStr> {
        iter::chain(self.reserved.iter(), self.names.iter())
    }
}

pub(crate) fn identifier_is_binding(identifier: &str) -> bool {
    normalize_identifier(identifier) == identifier && !identifier_is_reserved(identifier)
}

fn normalize_identifier(preferred: &str) -> String {
    let mut normalized = String::new();
    for (position, character) in preferred.chars().enumerate() {
        let valid_initial = character.is_ascii_alphabetic() || character == '_' || character == '$';
        let valid_subsequent = valid_initial || character.is_ascii_digit();
        if position == 0 && !valid_initial {
            normalized.push_str("value_");
        }
        if valid_subsequent {
            normalized.push(character);
        } else {
            normalized.push('_');
        }
    }
    if normalized.is_empty() {
        normalized.push_str("value");
    }
    normalized
}

fn identifier_is_reserved(identifier: &str) -> bool {
    matches!(
        identifier,
        "Array"
            | "Error"
            | "arguments"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "eval"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

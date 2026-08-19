use rustc_hash::FxHashSet;

#[derive(Debug, Default)]
pub(super) struct NameAllocator {
    names: FxHashSet<String>,
}

impl NameAllocator {
    pub(super) fn with_reserved(names: impl IntoIterator<Item = String>) -> NameAllocator {
        let names = names.into_iter();
        let names = names.collect();
        NameAllocator { names }
    }

    pub(super) fn allocate(&mut self, preferred: &str) -> String {
        let mut normalized = normalize_identifier(preferred);
        if identifier_is_reserved(&normalized) {
            normalized.insert(0, '$');
        }
        let mut candidate = normalized.clone();
        let mut suffix = 1;
        while !self.names.insert(candidate.clone()) {
            candidate = format!("{normalized}${suffix}");
            suffix += 1;
        }
        candidate
    }

    pub(super) fn allocated_names(&self) -> impl Iterator<Item = &String> {
        self.names.iter()
    }
}

pub(super) fn identifier_is_binding(identifier: &str) -> bool {
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
        "arguments"
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

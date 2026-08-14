const RESERVED_NAMES: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "await",
    "let",
    "static",
    "yield",
    "enum",
    "implements",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "abstract",
    "boolean",
    "byte",
    "char",
    "double",
    "final",
    "float",
    "goto",
    "int",
    "long",
    "native",
    "short",
    "synchronized",
    "throws",
    "transient",
    "volatile",
    "null",
    "true",
    "false",
];

const BUILT_IN_NAMES: &[&str] = &[
    "arguments",
    "Array",
    "ArrayBuffer",
    "Boolean",
    "DataView",
    "Date",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "Error",
    "escape",
    "eval",
    "EvalError",
    "Float32Array",
    "Float64Array",
    "Function",
    "Infinity",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "Intl",
    "isFinite",
    "isNaN",
    "JSON",
    "Map",
    "Math",
    "NaN",
    "Number",
    "Object",
    "parseFloat",
    "parseInt",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Set",
    "SIMD",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "undefined",
    "unescape",
    "URIError",
    "WeakMap",
    "WeakSet",
];

pub(crate) fn identifier_to_javascript(identifier: &str) -> String {
    if identifier.chars().next().is_some_and(char::is_numeric) {
        return format!("$${}", encode_identifier(identifier));
    }
    any_name_to_javascript(identifier)
}

pub(crate) fn any_name_to_javascript(name: &str) -> String {
    if is_reserved(name) || is_built_in(name) {
        format!("$${name}")
    } else {
        encode_identifier(name)
    }
}

pub(crate) fn module_name_to_javascript(module_name: &str) -> String {
    let name = module_name.replace('.', "_");
    if is_built_in(&name) { format!("$${name}") } else { name }
}

pub(crate) fn is_valid_javascript_identifier(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_alphabetic) && any_name_to_javascript(name) == name
}

pub(crate) fn exported_identifier(identifier: &str, local: bool) -> String {
    if local && (is_reserved(identifier) || is_built_in(identifier)) {
        format!("$${identifier} as {identifier}")
    } else if identifier == "$main" {
        format!("{} as $main", encode_identifier(identifier))
    } else {
        encode_identifier(identifier)
    }
}

fn is_reserved(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

fn is_built_in(name: &str) -> bool {
    BUILT_IN_NAMES.contains(&name)
}

fn encode_identifier(identifier: &str) -> String {
    let mut encoded = String::new();
    for character in identifier.chars() {
        match character {
            character if character.is_alphanumeric() => encoded.push(character),
            '_' => encoded.push('_'),
            '.' => encoded.push_str("$dot"),
            '$' => encoded.push_str("$dollar"),
            '~' => encoded.push_str("$tilde"),
            '=' => encoded.push_str("$eq"),
            '<' => encoded.push_str("$less"),
            '>' => encoded.push_str("$greater"),
            '!' => encoded.push_str("$bang"),
            '#' => encoded.push_str("$hash"),
            '%' => encoded.push_str("$percent"),
            '^' => encoded.push_str("$up"),
            '&' => encoded.push_str("$amp"),
            '|' => encoded.push_str("$bar"),
            '*' => encoded.push_str("$times"),
            '/' => encoded.push_str("$div"),
            '+' => encoded.push_str("$plus"),
            '-' => encoded.push_str("$minus"),
            ':' => encoded.push_str("$colon"),
            '\\' => encoded.push_str("$bslash"),
            '?' => encoded.push_str("$qmark"),
            '@' => encoded.push_str("$at"),
            '\'' => encoded.push_str("$prime"),
            character => encoded.push_str(&format!("${}", character as u32)),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{exported_identifier, identifier_to_javascript, module_name_to_javascript};

    #[test]
    fn matches_purescript_identifier_encoding() {
        assert_eq!(identifier_to_javascript("case"), "$$case");
        assert_eq!(identifier_to_javascript("Array"), "$$Array");
        assert_eq!(identifier_to_javascript("1st"), "$$1st");
        assert_eq!(identifier_to_javascript("<$>"), "$less$dollar$greater");
        assert_eq!(identifier_to_javascript("don't"), "don$primet");
        assert_eq!(module_name_to_javascript("Data.Array"), "Data_Array");
        assert_eq!(module_name_to_javascript("Array"), "$$Array");
        assert_eq!(exported_identifier("case", true), "$$case as case");
        assert_eq!(exported_identifier("$main", true), "$dollarmain as $main");
    }
}

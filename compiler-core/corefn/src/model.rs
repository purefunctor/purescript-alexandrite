use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub source_span: SourceSpan,
    pub module_name: ModuleName,
    pub module_path: String,
    pub imports: Vec<Import>,
    pub exports: Vec<Identifier>,
    pub re_exports: BTreeMap<String, Vec<Identifier>>,
    pub foreign: Vec<Identifier>,
    pub decls: Vec<Bind>,
    pub built_with: String,
    pub comments: Vec<Comment>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleName(pub Vec<String>);

impl ModuleName {
    pub fn from_dotted(name: &str) -> ModuleName {
        let components = name.split('.').map(str::to_owned);
        ModuleName(components.collect())
    }

    pub fn to_dotted(&self) -> String {
        self.0.join(".")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Identifier(pub String);

impl From<&str> for Identifier {
    fn from(value: &str) -> Identifier {
        Identifier(value.to_owned())
    }
}

impl From<String> for Identifier {
    fn from(value: String) -> Identifier {
        Identifier(value)
    }
}

/// A PureScript string, including values containing unpaired UTF-16 surrogates.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PureScriptString {
    String(String),
    CodeUnits(Vec<u16>),
}

impl From<&str> for PureScriptString {
    fn from(value: &str) -> PureScriptString {
        PureScriptString::String(value.to_owned())
    }
}

impl From<String> for PureScriptString {
    fn from(value: String) -> PureScriptString {
        PureScriptString::String(value)
    }
}

/// A JSON number with reflexive, bitwise equality for use in cached query values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoreFnNumber(pub f64);

impl PartialEq for CoreFnNumber {
    fn eq(&self, other: &CoreFnNumber) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for CoreFnNumber {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    pub annotation: Annotation,
    pub module_name: ModuleName,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comment {
    LineComment(String),
    BlockComment(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub const fn null() -> SourceSpan {
        let position = SourcePosition([0, 0]);
        SourceSpan { start: position, end: position }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourcePosition(pub [u32; 2]);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub source_span: SourceSpan,
    pub meta: Option<Meta>,
}

impl Annotation {
    pub fn new(source_span: SourceSpan) -> Annotation {
        Annotation { source_span, meta: None }
    }

    pub fn with_meta(source_span: SourceSpan, meta: Meta) -> Annotation {
        Annotation { source_span, meta: Some(meta) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "metaType")]
pub enum Meta {
    IsConstructor {
        #[serde(rename = "constructorType")]
        constructor_type: ConstructorType,
        identifiers: Vec<Identifier>,
    },
    IsNewtype,
    IsTypeClassConstructor,
    IsForeign,
    IsWhere,
    IsSyntheticApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructorType {
    ProductType,
    SumType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Qualified<T> {
    ByModuleName {
        #[serde(rename = "moduleName")]
        module_name: ModuleName,
        identifier: T,
    },
    BySourcePosition {
        #[serde(rename = "sourcePos")]
        source_position: SourcePosition,
        identifier: T,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "literalType", content = "value")]
pub enum Literal<T> {
    IntLiteral(i32),
    NumberLiteral(CoreFnNumber),
    StringLiteral(PureScriptString),
    CharLiteral(char),
    BooleanLiteral(bool),
    ArrayLiteral(Vec<T>),
    ObjectLiteral(Vec<(PureScriptString, T)>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "bindType")]
pub enum Bind {
    NonRec { annotation: Annotation, identifier: Identifier, expression: Expression },
    Rec { binds: Vec<RecBind> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecBind {
    pub identifier: Identifier,
    pub annotation: Annotation,
    pub expression: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Expression {
    Var {
        annotation: Annotation,
        value: Qualified<Identifier>,
    },
    Literal {
        annotation: Annotation,
        value: Literal<Box<Expression>>,
    },
    Constructor {
        annotation: Annotation,
        #[serde(rename = "typeName")]
        type_name: String,
        #[serde(rename = "constructorName")]
        constructor_name: String,
        #[serde(rename = "fieldNames")]
        field_names: Vec<Identifier>,
    },
    Accessor {
        annotation: Annotation,
        #[serde(rename = "fieldName")]
        field_name: PureScriptString,
        expression: Box<Expression>,
    },
    ObjectUpdate {
        annotation: Annotation,
        expression: Box<Expression>,
        copy: Option<Vec<PureScriptString>>,
        updates: Vec<(PureScriptString, Expression)>,
    },
    Abs {
        annotation: Annotation,
        argument: Identifier,
        body: Box<Expression>,
    },
    App {
        annotation: Annotation,
        abstraction: Box<Expression>,
        argument: Box<Expression>,
    },
    Case {
        annotation: Annotation,
        #[serde(rename = "caseExpressions")]
        case_expressions: Vec<Expression>,
        #[serde(rename = "caseAlternatives")]
        case_alternatives: Vec<CaseAlternative>,
    },
    Let {
        annotation: Annotation,
        binds: Vec<Bind>,
        expression: Box<Expression>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CaseAlternative {
    Unguarded {
        binders: Vec<Binder>,
        #[serde(rename = "isGuarded")]
        is_guarded: bool,
        expression: Expression,
    },
    Guarded {
        binders: Vec<Binder>,
        #[serde(rename = "isGuarded")]
        is_guarded: bool,
        expressions: Vec<GuardedExpression>,
    },
}

impl CaseAlternative {
    pub fn unguarded(binders: Vec<Binder>, expression: Expression) -> CaseAlternative {
        CaseAlternative::Unguarded { binders, is_guarded: false, expression }
    }

    pub fn guarded(binders: Vec<Binder>, expressions: Vec<GuardedExpression>) -> CaseAlternative {
        CaseAlternative::Guarded { binders, is_guarded: true, expressions }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardedExpression {
    pub guard: Expression,
    pub expression: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "binderType")]
pub enum Binder {
    VarBinder {
        annotation: Annotation,
        identifier: Identifier,
    },
    NullBinder {
        annotation: Annotation,
    },
    LiteralBinder {
        annotation: Annotation,
        literal: Literal<Box<Binder>>,
    },
    ConstructorBinder {
        annotation: Annotation,
        #[serde(rename = "typeName")]
        type_name: Qualified<String>,
        #[serde(rename = "constructorName")]
        constructor_name: Qualified<String>,
        binders: Vec<Binder>,
    },
    NamedBinder {
        annotation: Annotation,
        identifier: Identifier,
        binder: Box<Binder>,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Annotation, Bind, Binder, CaseAlternative, ConstructorType, CoreFnNumber, Expression,
        Identifier, Literal, Meta, Module, ModuleName, PureScriptString, Qualified, SourcePosition,
        SourceSpan,
    };

    fn annotation() -> Annotation {
        let source_span = SourceSpan { start: SourcePosition([1, 1]), end: SourcePosition([1, 2]) };
        Annotation::new(source_span)
    }

    #[test]
    fn matches_purescript_corefn_json_encoding() {
        let annotation = annotation();
        let variable = Expression::Var {
            annotation: annotation.clone(),
            value: Qualified::BySourcePosition {
                source_position: SourcePosition([1, 1]),
                identifier: Identifier::from("x"),
            },
        };
        let expression = Expression::Case {
            annotation: annotation.clone(),
            case_expressions: vec![variable.clone()],
            case_alternatives: vec![CaseAlternative::unguarded(
                vec![Binder::LiteralBinder {
                    annotation: annotation.clone(),
                    literal: Literal::BooleanLiteral(true),
                }],
                variable,
            )],
        };
        let module = Module {
            source_span: annotation.source_span.clone(),
            module_name: ModuleName::from_dotted("Test.Main"),
            module_path: "src/Test/Main.purs".to_owned(),
            imports: vec![],
            exports: vec![Identifier::from("main")],
            re_exports: Default::default(),
            foreign: vec![],
            decls: vec![Bind::NonRec {
                annotation: Annotation::with_meta(
                    annotation.source_span.clone(),
                    Meta::IsConstructor {
                        constructor_type: ConstructorType::ProductType,
                        identifiers: vec![Identifier::from("value0")],
                    },
                ),
                identifier: Identifier::from("main"),
                expression,
            }],
            built_with: "0.15.15".to_owned(),
            comments: vec![],
        };

        let value = serde_json::to_value(&module).unwrap();
        assert_eq!(value["moduleName"], json!(["Test", "Main"]));
        assert_eq!(value["decls"][0]["bindType"], "NonRec");
        assert_eq!(value["decls"][0]["annotation"]["meta"]["metaType"], "IsConstructor");
        assert_eq!(value["decls"][0]["expression"]["type"], "Case");
        assert_eq!(value["decls"][0]["expression"]["caseAlternatives"][0]["isGuarded"], false);
        assert!(
            value["decls"][0]["expression"]["caseAlternatives"][0].get("expressions").is_none()
        );

        let decoded: Module = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, module);
    }

    #[test]
    fn decodes_purescript_strings_with_unpaired_surrogates() {
        let value = json!({
            "literalType": "StringLiteral",
            "value": [0x0061, 0xd800, 0x0062]
        });
        let literal: Literal<()> = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(
            literal,
            Literal::StringLiteral(PureScriptString::CodeUnits(vec![0x0061, 0xd800, 0x0062]))
        );
        assert_eq!(serde_json::to_value(literal).unwrap(), value);
    }

    #[test]
    fn round_trips_purescript_numbers() {
        for source in ["3.40282347e+38", "8639977881599999.0", "1.38064852e-23", "1e-28"] {
            let number: CoreFnNumber = serde_json::from_str(source).unwrap();
            let encoded = serde_json::to_string(&number).unwrap();
            let decoded: CoreFnNumber = serde_json::from_str(&encoded).unwrap();

            assert_eq!(decoded, number, "{source} encoded as {encoded}");
        }
    }
}

use std::sync::Arc;

use lexing::Lexed;
use syntax::{ElementCategory, SyntaxKind, SyntaxValue, TreeOwner};
use syntree::Builder as SyntreeBuilder;

use crate::{ParseError, ParsedModule};

#[derive(Debug)]
pub(crate) enum Output {
    Start { kind: SyntaxKind },
    Annotate,
    Qualify,
    Token { kind: SyntaxKind },
    Error { message: ParserError },
    Finish,
}

#[derive(Debug)]
pub(crate) enum ParserError {
    Message(&'static str),
    Expected(SyntaxKind),
}

struct Builder<'l, 's> {
    lexed: &'l Lexed<'s>,
    index: usize,
    annotated: bool,
    qualified: bool,
    builder: SyntreeBuilder<SyntaxValue>,
    errors: Vec<ParseError>,
}

impl<'l, 's> Builder<'l, 's> {
    fn new(lexed: &'l Lexed<'s>) -> Builder<'l, 's> {
        let index = 0;
        let annotated = false;
        let qualified = false;
        let builder = SyntreeBuilder::new();
        let errors = vec![];
        Builder { lexed, index, annotated, qualified, builder, errors }
    }

    fn build(self) -> (ParsedModule, Vec<ParseError>) {
        let tree = self.builder.build().expect("parser must produce a balanced syntax tree");
        (ParsedModule::new(TreeOwner::new(tree)), self.errors)
    }

    fn start(&mut self, kind: SyntaxKind) {
        if kind != SyntaxKind::Node {
            self.builder
                .open(SyntaxValue { kind, category: ElementCategory::Node })
                .expect("syntax tree capacity exceeded");
        }
    }

    fn annotate(&mut self) {
        if let Some(annotation) = self.lexed.annotation(self.index)
            && !self.annotated
        {
            self.start(SyntaxKind::Annotation);
            self.builder
                .token(
                    SyntaxValue { kind: SyntaxKind::TEXT, category: ElementCategory::Token },
                    annotation.len(),
                )
                .expect("syntax tree capacity exceeded");
            self.finish();
        }

        self.annotated = true;
    }

    fn qualify(&mut self) {
        if let Some(qualifier) = self.lexed.qualifier(self.index)
            && !self.qualified
        {
            self.start(SyntaxKind::Qualifier);
            self.builder
                .token(
                    SyntaxValue { kind: SyntaxKind::TEXT, category: ElementCategory::Token },
                    qualifier.len(),
                )
                .expect("syntax tree capacity exceeded");
            self.finish();
        }

        self.qualified = true;
    }

    fn token(&mut self, kind: SyntaxKind) {
        if kind.is_layout_token() {
            self.builder
                .token_empty(SyntaxValue { kind, category: ElementCategory::Token })
                .expect("syntax tree capacity exceeded");
            return;
        }

        self.annotate();

        if let Some(message) = self.lexed.error(self.index) {
            self.error(message);
        }

        self.qualify();

        if !matches!(kind, SyntaxKind::ERROR) {
            let text = self.lexed.text(self.index);
            self.builder
                .token(SyntaxValue { kind, category: ElementCategory::Token }, text.len())
                .expect("syntax tree capacity exceeded");
        }

        self.index += 1;
        self.annotated = false;
        self.qualified = false;
    }

    fn error(&mut self, message: impl Into<Arc<str>>) {
        let info = self.lexed.info(self.index);
        let offset = info.qualifier as usize;
        let position = self.lexed.position(self.index);
        let message = message.into();
        self.builder
            .token_empty(SyntaxValue { kind: SyntaxKind::ERROR, category: ElementCategory::Token })
            .expect("syntax tree capacity exceeded");
        self.errors.push(ParseError { offset, position, message });
    }

    fn finish(&mut self) {
        self.builder.close().expect("parser must produce a balanced syntax tree");
    }
}

pub(crate) fn build(lexed: &Lexed<'_>, output: Vec<Output>) -> (ParsedModule, Vec<ParseError>) {
    let mut builder = Builder::new(lexed);

    for event in output {
        match event {
            Output::Start { kind } => builder.start(kind),
            Output::Annotate => builder.annotate(),
            Output::Qualify => builder.qualify(),
            Output::Token { kind } => builder.token(kind),
            Output::Error { message: ParserError::Message(message) } => builder.error(message),
            Output::Error { message: ParserError::Expected(kind) } => {
                builder.error(format!("Expected {kind:?}"));
            }
            Output::Finish => builder.finish(),
        }
    }

    builder.build()
}

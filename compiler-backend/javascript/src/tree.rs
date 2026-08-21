use la_arena::{Arena, Idx};

pub(crate) type ExpressionId = Idx<Expression>;

#[derive(Debug, Default)]
pub(crate) struct Tree {
    expressions: Arena<Expression>,
}

#[derive(Debug)]
pub(crate) enum Expression {
    Identifier(String),
    String(String),
    Number(String),
    Boolean(bool),
    Array(Vec<ExpressionId>),
    Object(Vec<ObjectProperty>),
    Call { callee: ExpressionId, arguments: Vec<ExpressionId> },
    Member { object: ExpressionId, property: String },
    Index { object: ExpressionId, index: ExpressionId },
    Binary { operator: BinaryOperator, left: ExpressionId, right: ExpressionId },
    Arrow { parameters: Vec<String>, body: ExpressionId },
}

#[derive(Debug)]
pub(crate) enum ObjectProperty {
    Field { name: String, value: ExpressionId },
    Spread(ExpressionId),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BinaryOperator {
    StrictEqual,
    BitwiseOr,
    LogicalAnd,
}

impl Tree {
    pub(crate) fn allocate(&mut self, expression: Expression) -> ExpressionId {
        self.expressions.alloc(expression)
    }

    pub(crate) fn identifier(&mut self, name: impl Into<String>) -> ExpressionId {
        self.allocate(Expression::Identifier(name.into()))
    }

    pub(crate) fn string(&mut self, value: impl Into<String>) -> ExpressionId {
        self.allocate(Expression::String(value.into()))
    }

    pub(crate) fn number(&mut self, value: impl Into<String>) -> ExpressionId {
        self.allocate(Expression::Number(value.into()))
    }

    pub(crate) fn boolean(&mut self, value: bool) -> ExpressionId {
        self.allocate(Expression::Boolean(value))
    }

    pub(crate) fn array(&mut self, elements: Vec<ExpressionId>) -> ExpressionId {
        self.allocate(Expression::Array(elements))
    }

    pub(crate) fn object(&mut self, properties: Vec<ObjectProperty>) -> ExpressionId {
        self.allocate(Expression::Object(properties))
    }

    pub(crate) fn call(
        &mut self,
        callee: ExpressionId,
        arguments: Vec<ExpressionId>,
    ) -> ExpressionId {
        self.allocate(Expression::Call { callee, arguments })
    }

    pub(crate) fn member(
        &mut self,
        object: ExpressionId,
        property: impl Into<String>,
    ) -> ExpressionId {
        self.allocate(Expression::Member { object, property: property.into() })
    }

    pub(crate) fn index(&mut self, object: ExpressionId, index: ExpressionId) -> ExpressionId {
        self.allocate(Expression::Index { object, index })
    }

    pub(crate) fn binary(
        &mut self,
        operator: BinaryOperator,
        left: ExpressionId,
        right: ExpressionId,
    ) -> ExpressionId {
        self.allocate(Expression::Binary { operator, left, right })
    }

    pub(crate) fn arrow(&mut self, parameters: Vec<String>, body: ExpressionId) -> ExpressionId {
        self.allocate(Expression::Arrow { parameters, body })
    }
}

impl std::ops::Index<ExpressionId> for Tree {
    type Output = Expression;

    fn index(&self, expression: ExpressionId) -> &Expression {
        &self.expressions[expression]
    }
}

use std::sync::Arc;

use crate::context::CheckContext;
use crate::core::TypeId;
use crate::source::terms::ElaboratedExpression;
use crate::state::CheckState;
use crate::{ExternalQueries, tree};

pub struct DerivedTreeBuilder<'state, 'context, 'query, Q: ExternalQueries> {
    state: &'state mut CheckState,
    context: &'context CheckContext<'query, Q>,
    derive_id: indexing::DeriveId,
}

impl<'state, 'context, 'query, Q> DerivedTreeBuilder<'state, 'context, 'query, Q>
where
    Q: ExternalQueries,
{
    pub fn new(
        state: &'state mut CheckState,
        context: &'context CheckContext<'query, Q>,
        derive_id: indexing::DeriveId,
    ) -> DerivedTreeBuilder<'state, 'context, 'query, Q> {
        DerivedTreeBuilder { state, context, derive_id }
    }

    pub fn variable_binder(&mut self, name: &str, type_id: TypeId) -> tree::BinderId {
        self.allocate_binder(name, type_id, tree::BinderKind::Variable)
    }

    pub fn constructor_pattern(
        &mut self,
        name: &str,
        type_id: TypeId,
        resolution: (files::FileId, indexing::TermItemId),
        arguments: Vec<tree::BinderId>,
    ) -> tree::BinderId {
        let kind = tree::BinderKind::Constructor { resolution, arguments: Arc::from(arguments) };
        self.allocate_binder(name, type_id, kind)
    }

    pub fn variable(&mut self, binder: tree::BinderId) -> ElaboratedExpression {
        let type_id = self.state.checked.tree[binder].type_id;
        let resolution = tree::VariableResolution::Generated(binder);
        self.allocate_expression(type_id, tree::ExpressionKind::Variable { resolution })
    }

    pub fn boolean(&mut self, type_id: TypeId, value: bool) -> ElaboratedExpression {
        self.allocate_expression(type_id, tree::ExpressionKind::Boolean { value })
    }

    pub fn alternative(
        &self,
        patterns: Vec<tree::BinderId>,
        body: ElaboratedExpression,
    ) -> tree::CaseAlternative {
        let where_expression = tree::WhereExpression::new(body.expression);
        let guarded_expression = tree::GuardedExpression::unconditional(where_expression);
        tree::CaseAlternative { binders: Arc::from(patterns), guarded_expression }
    }

    pub fn case(
        &mut self,
        result_type: TypeId,
        scrutinees: Vec<ElaboratedExpression>,
        alternatives: Vec<tree::CaseAlternative>,
    ) -> ElaboratedExpression {
        let scrutinees = scrutinees.into_iter().map(|scrutinee| scrutinee.expression);
        let scrutinees = scrutinees.collect();
        let kind = tree::ExpressionKind::Case { scrutinees, alternatives: Arc::from(alternatives) };
        self.allocate_expression(result_type, kind)
    }

    pub fn lambda(
        &mut self,
        implementation_type: TypeId,
        binders: Vec<tree::BinderId>,
        body: ElaboratedExpression,
    ) -> ElaboratedExpression {
        let kind = tree::ExpressionKind::Lambda {
            binders: Arc::from(binders),
            expression: body.expression,
        };
        self.allocate_expression(implementation_type, kind)
    }

    fn allocate_binder(
        &mut self,
        name: &str,
        type_id: TypeId,
        kind: tree::BinderKind,
    ) -> tree::BinderId {
        let name = self.context.queries.intern_smol_str(name.into());
        self.state.allocate_derived_binder(self.derive_id, name, type_id, kind)
    }

    fn allocate_expression(
        &mut self,
        type_id: TypeId,
        kind: tree::ExpressionKind,
    ) -> ElaboratedExpression {
        let expression = self.state.allocate_expression(type_id, kind);
        ElaboratedExpression { type_id, expression }
    }
}

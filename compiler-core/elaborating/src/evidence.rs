use checking::evidence::{Evidence, EvidenceId, EvidenceState, EvidenceVarId, Evidences};

use crate::{CoreError, CoreExpression, CoreExpressionId, CoreModule, CoreVariable};

/// Turns solved checker evidence into ordinary Core expressions.
///
/// `None` means compiler-known evidence and is intentionally erased. Every
/// other state, including checker recovery, has a concrete Core result.
pub(crate) struct EvidenceResolver<'a, 'module> {
    evidence: &'a Evidences,
    module: &'module mut CoreModule,
    resolving: Vec<EvidenceVarId>,
}

impl<'a, 'module> EvidenceResolver<'a, 'module> {
    pub(crate) fn new(evidence: &'a Evidences, module: &'module mut CoreModule) -> Self {
        EvidenceResolver { evidence, module, resolving: Vec::new() }
    }

    pub(crate) fn resolve_variable(&mut self, variable: EvidenceVarId) -> Option<CoreExpressionId> {
        if self.resolving.contains(&variable) {
            return Some(self.error());
        }

        self.resolving.push(variable);
        let resolved = match self.evidence.variable(variable).state {
            EvidenceState::Solved(evidence) => self.resolve_term(evidence),
            EvidenceState::Error | EvidenceState::Unsolved => Some(self.error()),
        };
        self.resolving.pop();
        resolved
    }

    fn resolve_term(&mut self, id: EvidenceId) -> Option<CoreExpressionId> {
        match self.evidence.evidence(id) {
            Evidence::Variable(variable) => self.resolve_variable(*variable),
            Evidence::Given(binder) => {
                if self.evidence.binder(*binder).erased {
                    return None;
                }
                Some(
                    self.module.allocate_expression(CoreExpression::Variable(
                        CoreVariable::Evidence(*binder),
                    )),
                )
            }
            Evidence::Instance { origin, subgoals } => {
                let mut expression = self
                    .module
                    .allocate_expression(CoreExpression::Variable(CoreVariable::Instance(*origin)));

                for &subgoal in subgoals {
                    let Some(argument) = self.resolve_variable(subgoal) else {
                        continue;
                    };
                    expression = self.module.allocate_expression(CoreExpression::Apply {
                        function: expression,
                        argument,
                    });
                }
                Some(expression)
            }
            Evidence::Superclass { parent, index } => {
                let dictionary = self.resolve_term(*parent).unwrap_or_else(|| self.error());
                Some(self.module.allocate_expression(CoreExpression::SuperclassProjection {
                    dictionary,
                    index: *index,
                }))
            }
            Evidence::Compiler => None,
        }
    }

    fn error(&mut self) -> CoreExpressionId {
        self.module.allocate_expression(CoreExpression::Error(CoreError::Evidence))
    }
}

#[cfg(test)]
mod tests {
    use checking::evidence::{Evidence, EvidenceState, Evidences};

    use super::EvidenceResolver;
    use crate::{CoreExpression, CoreModule, CoreVariable};

    #[test]
    fn aliases_resolve_to_the_given_dictionary() {
        let mut evidence = Evidences::default();
        let binder = evidence.fresh_binder();
        let representative = evidence.fresh_variable();
        let duplicate = evidence.fresh_variable();
        let given = evidence.given(binder);
        evidence.solve(representative, given);
        evidence.merge_duplicate(duplicate, representative);

        let mut module = CoreModule::default();
        let resolved = EvidenceResolver::new(&evidence, &mut module)
            .resolve_variable(duplicate)
            .expect("given evidence must be retained");

        assert_eq!(
            module.expressions[resolved],
            CoreExpression::Variable(CoreVariable::Evidence(binder))
        );
    }

    #[test]
    fn compiler_evidence_is_erased() {
        let mut evidence = Evidences::default();
        let variable = evidence.fresh_variable();
        let compiler = evidence.allocate(Evidence::Compiler);
        evidence.solve(variable, compiler);

        let mut module = CoreModule::default();
        assert_eq!(EvidenceResolver::new(&evidence, &mut module).resolve_variable(variable), None);
        assert!(module.expressions.is_empty());
    }

    #[test]
    fn errored_evidence_has_a_recovery_expression() {
        let mut evidence = Evidences::default();
        let variable = evidence.fresh_variable();
        evidence.mark_error(variable);
        assert_eq!(evidence.variable(variable).state, EvidenceState::Error);

        let mut module = CoreModule::default();
        assert!(EvidenceResolver::new(&evidence, &mut module).resolve_variable(variable).is_some());
    }
}

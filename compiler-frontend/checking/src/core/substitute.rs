//! Implements name-based type substitution for the core representation.

use rustc_hash::FxHashMap;

use building_types::QueryResult;

use crate::ExternalQueries;
use crate::context::CheckContext;
use crate::core::fold::{FoldAction, TypeFold, fold_type};
use crate::core::{Depth, Name, Type, TypeId};
use crate::state::CheckState;

pub type NameToType = FxHashMap<Name, TypeId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RigidReplacement {
    name: Name,
    depth: Depth,
    type_id: TypeId,
}

/// A name-based replacement of rigid variables with fresh rigid variables.
///
/// Unlike [`NameToType`], this representation cannot contain non-rigid
/// replacement types.
#[derive(Debug, Default)]
pub struct RigidRenaming {
    replacements: FxHashMap<Name, RigidReplacement>,
}

impl RigidRenaming {
    pub fn insert<Q>(&mut self, context: &CheckContext<Q>, original: Name, replacement: TypeId)
    where
        Q: ExternalQueries,
    {
        let Type::Rigid(name, depth, _) = context.lookup_type(replacement) else {
            unreachable!("invariant violated: expected a rigid variable");
        };
        let replacement = RigidReplacement { name, depth, type_id: replacement };
        self.replacements.insert(original, replacement);
    }

    pub fn substitute<Q>(
        &self,
        state: &mut CheckState,
        context: &CheckContext<Q>,
        in_type: TypeId,
    ) -> QueryResult<TypeId>
    where
        Q: ExternalQueries,
    {
        fold_type(state, context, in_type, &mut SubstituteRigidName { renaming: self })
    }

    pub(crate) fn replacement(&self, original: Name) -> Option<(Name, Depth)> {
        self.replacements.get(&original).map(|replacement| (replacement.name, replacement.depth))
    }
}

/// Implements [`Name`]-based substitution for [`Type::Rigid`] variables.
///
/// Names are globally unique, removing the need for scope tracking and
/// removing the need for capture-avoiding substitutions. This property
/// is extremely useful for for instantiation.
pub struct SubstituteName<'a> {
    bindings: &'a NameToType,
}

impl SubstituteName<'_> {
    pub fn one<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        name: Name,
        replacement: TypeId,
        in_type: TypeId,
    ) -> QueryResult<TypeId>
    where
        Q: ExternalQueries,
    {
        let bindings = NameToType::from_iter([(name, replacement)]);
        fold_type(state, context, in_type, &mut SubstituteName { bindings: &bindings })
    }

    pub fn many<Q>(
        state: &mut CheckState,
        context: &CheckContext<Q>,
        bindings: &NameToType,
        in_type: TypeId,
    ) -> QueryResult<TypeId>
    where
        Q: ExternalQueries,
    {
        fold_type(state, context, in_type, &mut SubstituteName { bindings })
    }
}

impl TypeFold for SubstituteName<'_> {
    fn transform<Q>(
        &mut self,
        _state: &mut CheckState,
        _context: &CheckContext<Q>,
        _id: TypeId,
        t: &Type,
    ) -> QueryResult<FoldAction>
    where
        Q: ExternalQueries,
    {
        if let Type::Rigid(name, _, _) = t
            && let Some(id) = self.bindings.get(name)
        {
            Ok(FoldAction::Replace(*id))
        } else {
            Ok(FoldAction::Continue)
        }
    }
}

struct SubstituteRigidName<'a> {
    renaming: &'a RigidRenaming,
}

impl TypeFold for SubstituteRigidName<'_> {
    fn transform<Q>(
        &mut self,
        _state: &mut CheckState,
        _context: &CheckContext<Q>,
        _id: TypeId,
        t: &Type,
    ) -> QueryResult<FoldAction>
    where
        Q: ExternalQueries,
    {
        if let Type::Rigid(name, _, _) = t
            && let Some(replacement) = self.renaming.replacements.get(name)
        {
            Ok(FoldAction::Replace(replacement.type_id))
        } else {
            Ok(FoldAction::Continue)
        }
    }
}

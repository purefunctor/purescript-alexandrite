use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::{any, fmt, hash};

use syntax::ast::{AstNode, AstPtr};

pub struct AstId<N: AstNode> {
    pub(crate) id: NonZeroU32,
    phantom: PhantomData<fn() -> AstPtr<N>>,
}

impl<N: AstNode> AstId<N> {
    pub const fn new(id: NonZeroU32) -> AstId<N> {
        AstId { id, phantom: PhantomData }
    }

    pub const fn into_raw(self) -> NonZeroU32 {
        self.id
    }
}

impl<N: AstNode> fmt::Debug for AstId<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("AstId<{}>({})", any::type_name::<N>(), self.id))
    }
}

impl<N: AstNode> Clone for AstId<N> {
    fn clone(&self) -> AstId<N> {
        *self
    }
}

impl<N: AstNode> Copy for AstId<N> {}

impl<N: AstNode> PartialEq for AstId<N> {
    fn eq(&self, other: &AstId<N>) -> bool {
        self.id == other.id
    }
}

impl<N: AstNode> Eq for AstId<N> {}

impl<N: AstNode> PartialOrd for AstId<N> {
    fn partial_cmp(&self, other: &AstId<N>) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<N: AstNode> Ord for AstId<N> {
    fn cmp(&self, other: &AstId<N>) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<N: AstNode> hash::Hash for AstId<N> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

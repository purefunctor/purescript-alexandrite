//! Functional identities for StyleX compiler intrinsics.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleXIntrinsic {
    Create,
    Props,
    RecordProps,
    Conditional,
    Keyframes,
}

impl StyleXIntrinsic {
    pub fn name(self) -> &'static str {
        match self {
            StyleXIntrinsic::Create => "create",
            StyleXIntrinsic::Props => "props",
            StyleXIntrinsic::RecordProps => "recordProps",
            StyleXIntrinsic::Conditional => "conditional",
            StyleXIntrinsic::Keyframes => "keyframes",
        }
    }
}

use files::FileId;
use thiserror::Error;

pub type ModuleResult<T> = Result<T, ModuleError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModuleError {
    #[error(transparent)]
    Functional(#[from] nbe::ModuleError),
    #[error("cannot convert functional module {file_id:?} to SSA: {state}")]
    Unsupported { file_id: FileId, state: UnsupportedState },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnsupportedState {
    #[error("local {local_name} is not available in the current lexical scope")]
    MissingLocal { local_name: String },
    #[error("case expression has no alternatives")]
    MissingCaseAlternative,
    #[error("guarded expression has no alternatives")]
    MissingGuardedAlternative,
    #[error("case alternative has {patterns} patterns for {scrutinees} scrutinees")]
    CaseArity { patterns: usize, scrutinees: usize },
    #[error("function {function_name} contains an unterminated basic block")]
    UnterminatedBlock { function_name: String },
}

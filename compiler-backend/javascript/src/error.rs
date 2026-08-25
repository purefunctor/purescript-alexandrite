use files::FileId;
use thiserror::Error;

pub type ModuleResult<T> = Result<T, ModuleError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModuleError {
    #[error(transparent)]
    Functional(#[from] nbe::ModuleError),
    #[error("cannot convert functional module {file_id:?} to JavaScript: {state}")]
    Unsupported { file_id: FileId, state: UnsupportedState },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnsupportedState {
    #[error("number literal {value:?} is not a finite JavaScript number")]
    InvalidNumber { value: String },
    #[error("top-level value initializers form a cycle")]
    CyclicInitializers,
    #[error("local global {name:?} has no JavaScript declaration")]
    MissingGlobal { name: String },
    #[error("local value {name:?} has no JavaScript binding")]
    MissingLocal { name: String },
}

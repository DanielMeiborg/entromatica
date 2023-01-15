use std::sync::{MutexGuard, PoisonError};

use backtrace::Backtrace as trc;
use thiserror::Error;

use crate::prelude::*;

#[derive(Debug, Clone, Error)]
#[error(transparent)]
pub struct InternalError(#[from] InternalErrorKind);

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
#[error(transparent)]
pub(crate) enum InternalErrorKind {
    CacheError(#[from] CacheError),
    ThreadingError(#[from] ThreadingError),
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum ErrorKind {
    #[error("EntityError: {0:#?}")]
    EntityError(#[from] EntityError),

    #[error("Internal error: {0:#?}")]
    InternalError(#[from] InternalError),

    #[error("PossibleStatesError: {0:#?}")]
    PossibleStatesError(#[from] PossibleStatesError),

    #[error("RuleError: {0:#?}")]
    RuleError(#[from] RuleError),

    #[error("UnitsError: {0:#?}")]
    UnitsError(#[from] UnitsError),

    #[error("StateError: {0:#?}")]
    StateError(#[from] StateError),

    #[error("ReachableStatesError: {0:#?}")]
    ReachableStatesError(#[from] ReachableStatesError),
}

impl From<CacheError> for ErrorKind {
    fn from(cache_error: CacheError) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::CacheError(cache_error)))
    }
}

impl From<PoisonError<MutexGuard<'_, PossibleStates>>> for ErrorKind {
    fn from(poison_error: PoisonError<MutexGuard<'_, PossibleStates>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::PossibleStatesSyncError {
                msg: format!("{:?}", poison_error),
                context: get_backtrace(),
            },
        )))
    }
}

impl From<PoisonError<MutexGuard<'_, ReachableStates>>> for ErrorKind {
    fn from(poison_error: PoisonError<MutexGuard<'_, ReachableStates>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::ReachableStatesSyncError {
                msg: format!("{:?}", poison_error),
                context: get_backtrace(),
            },
        )))
    }
}

impl From<PoisonError<MutexGuard<'_, Cache>>> for ErrorKind {
    fn from(poison_error: PoisonError<MutexGuard<'_, Cache>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::CacheSyncError {
                msg: format!("{:?}", poison_error),
                context: get_backtrace(),
            },
        )))
    }
}

#[derive(Debug, Clone, Error)]
pub enum ThreadingError {
    #[error("Error while syncing possible states: {msg:?}")]
    PossibleStatesSyncError { msg: String, context: trc },

    #[error("Error while syncing reachable states: {msg:?}")]
    ReachableStatesSyncError { msg: String, context: trc },

    #[error("Error while syncing cache: {msg:?}")]
    CacheSyncError { msg: String, context: trc },
}

#[cfg(debug_assertions)]
pub(crate) fn get_backtrace() -> trc {
    trc::new()
}

#[cfg(not(debug_assertions))]
pub(crate) fn get_backtrace() -> trc {
    trc::new_unresolved()
}

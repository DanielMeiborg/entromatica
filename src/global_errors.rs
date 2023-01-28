use std::fmt::Debug;
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
pub enum ErrorKind<T: Debug + Clone> {
    #[error("EntityError: {0:#?}")]
    EntityError(#[from] EntityError),

    #[error("Internal error: {0:#?}")]
    InternalError(#[from] InternalError),

    #[error("PossibleStatesError: {0:#?}")]
    PossibleStatesError(#[from] PossibleStatesError<T>),

    #[error("RuleError: {0:#?}")]
    RuleError(#[from] RuleError),

    #[error("UnitsError: {0:#?}")]
    UnitsError(#[from] UnitsError),

    #[error("StateError: {0:#?}")]
    StateError(#[from] StateError),

    #[error("ReachableStatesError: {0:#?}")]
    ReachableStatesError(#[from] ReachableStatesError),

    #[error("The simulation has reached the iteration limit of {time} timesteps.")]
    IterationLimitReached { time: usize, context: trc },
}

impl<T: Clone + Debug> From<CacheError> for ErrorKind<T> {
    fn from(cache_error: CacheError) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::CacheError(cache_error)))
    }
}

impl<T: Clone + Debug> From<PoisonError<MutexGuard<'_, PossibleStates<T>>>> for ErrorKind<T> {
    fn from(poison_error: PoisonError<MutexGuard<'_, PossibleStates<T>>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::PossibleStatesSyncError {
                msg: format!("{poison_error:?}"),
                context: get_backtrace(),
            },
        )))
    }
}

impl<T: Clone + Debug> From<PoisonError<MutexGuard<'_, ReachableStates>>> for ErrorKind<T> {
    fn from(poison_error: PoisonError<MutexGuard<'_, ReachableStates>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::ReachableStatesSyncError {
                msg: format!("{poison_error:?}"),
                context: get_backtrace(),
            },
        )))
    }
}

impl<T: Clone + Debug> From<PoisonError<MutexGuard<'_, Cache>>> for ErrorKind<T> {
    fn from(poison_error: PoisonError<MutexGuard<'_, Cache>>) -> Self {
        Self::InternalError(InternalError(InternalErrorKind::ThreadingError(
            ThreadingError::CacheSyncError {
                msg: format!("{poison_error:?}"),
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

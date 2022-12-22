use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::mpsc::SendError;

use crate::cache::*;
use crate::resource::*;
use crate::rules::*;
use crate::state::*;
use crate::units::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AlreadyExistsError<O: Debug, C: Debug> {
    object: O,
    container: C,
}

impl<O: Debug, C: Debug> Display for AlreadyExistsError<O, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Object {:#?} already exists in container {:#?}",
            self.object, self.container
        )
    }
}

impl<O: Debug, C: Debug> Error for AlreadyExistsError<O, C> {}

impl<O: Debug, C: Debug> AlreadyExistsError<O, C> {
    pub fn new(object: O, container: C) -> Self {
        Self { object, container }
    }

    pub fn object(&self) -> &O {
        &self.object
    }

    pub fn container(&self) -> &C {
        &self.container
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct NotFoundError<O: Debug, C: Debug> {
    object: O,
    container: C,
}

impl<O: Debug, C: Debug> Display for NotFoundError<O, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Object {:#?} not found in container {:#?}",
            self.object, self.container
        )
    }
}

impl<O: Debug, C: Debug> Error for NotFoundError<O, C> {}

impl<O: Debug, C: Debug> NotFoundError<O, C> {
    pub fn new(object: O, container: C) -> Self {
        Self { object, container }
    }

    pub fn object(&self) -> &O {
        &self.object
    }

    pub fn container(&self) -> &C {
        &self.container
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OutOfRangeError<T: Debug> {
    object: T,
    lower_bound: T,
    upper_bound: T,
}

impl<T: Debug> Display for OutOfRangeError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Object {:#?} not in range [{:#?}, {:#?}]",
            self.object, self.lower_bound, self.upper_bound
        )
    }
}

impl<T: Debug> Error for OutOfRangeError<T> {}

impl<T: Debug> OutOfRangeError<T> {
    pub fn new(object: T, lower_bound: T, upper_bound: T) -> Self {
        Self {
            object,
            lower_bound,
            upper_bound,
        }
    }

    pub fn object(&self) -> &T {
        &self.object
    }

    pub fn lower_bound(&self) -> &T {
        &self.lower_bound
    }

    pub fn upper_bound(&self) -> &T {
        &self.upper_bound
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct InternalError {
    message: String,
}

impl Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Internal error: {}", self.message)
    }
}

impl Error for InternalError {}

impl InternalError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn from_error<E: Debug>(error: E) -> Self {
        Self {
            message: format!("{:#?}", error),
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InternalErrorKind {
    ConditionAlreadyExists(AlreadyExistsError<(StateHash, RuleApplies), Cache>),
    ActionAlreadyExists(AlreadyExistsError<(StateHash, StateHash), Cache>),
    RuleAlreadyExists(AlreadyExistsError<RuleName, Cache>),
    RuleNotFound(NotFoundError<RuleName, Cache>),
    ConditionCacheUpdateSendError(SendError<ConditionCacheUpdate>),
    ActionCacheUpdateSendError(SendError<ActionCacheUpdate>),
}

impl InternalErrorKind {
    pub(crate) fn to_error_kind(&self) -> ErrorKind {
        ErrorKind::InternalError(InternalError::from_error(self))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    ResourceNotFound(NotFoundError<ResourceName, Entity>),
    StateInPossibleStatesNotFound(NotFoundError<StateHash, PossibleStates>),
    StateInReachableStatesNotFound(NotFoundError<StateHash, ReachableStates>),
    EntityNotFound(NotFoundError<EntityName, State>),
    AmountExceedsEntityLimit(OutOfRangeError<Amount>),
    TotalAmountExceedsResourceLimit(OutOfRangeError<Amount>),
    AmountIsNegative(OutOfRangeError<Amount>),
    ProbabilityOutOfRange(OutOfRangeError<Probability>),
    InternalError(InternalError),
    StateAlreadyExists(AlreadyExistsError<StateHash, State>),
}

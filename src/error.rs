use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::resource::*;
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

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceCapacityError {
    NotFound(NotFoundError<ResourceName, (EntityName, Entity)>),
    OutOfRange(OutOfRangeError<Amount>),
}

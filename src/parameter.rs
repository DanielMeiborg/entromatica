use std::fmt::Display;

use derive_more::*;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};

use backtrace::Backtrace as trc;
use thiserror::Error;

#[allow(unused_imports)]
use crate::prelude::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, AsRef, AsMut, Into)]
pub struct ParameterName(String);

impl ParameterName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

#[derive(PartialEq, Clone, Debug, Default, From, Into)]
pub struct Parameter {
    description: String,
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parameter: {}", self.description)?;
        Ok(())
    }
}

impl Parameter {
    pub fn new() -> Self {
        Self {
            description: "".to_string(),
        }
    }

    pub fn from(description: String) -> Self {
        Self { description }
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum ParameterError {
    #[error("Parameter not found: {parameter_name:#?}")]
    ParameterNotFound {
        parameter_name: ParameterName,
        context: trc,
    },
}

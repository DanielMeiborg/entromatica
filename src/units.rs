use std::hash::{Hash, Hasher};

use backtrace::Backtrace as trc;
use derive_more::*;
use thiserror::Error;

use crate::prelude::*;

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    Into,
    AsRef,
    AsMut,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Amount(f64);

impl Hash for Amount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl From<f64> for Amount {
    fn from(amount: f64) -> Self {
        Self(amount)
    }
}

impl Amount {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_f64(&self) -> f64 {
        self.0
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    Into,
    AsRef,
    AsMut,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Entropy(f64);

impl Hash for Entropy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Entropy {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl From<f64> for Entropy {
    fn from(entropy: f64) -> Self {
        debug_assert!(entropy >= 0.);
        Self(entropy)
    }
}

impl Entropy {
    pub fn new() -> Self {
        Self(0.)
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    Into,
    AsRef,
    AsMut,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Probability(f64);

impl Hash for Probability {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Probability {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl From<f64> for Probability {
    fn from(probability: f64) -> Self {
        debug_assert!((0. ..=1.).contains(&probability));
        Self(probability)
    }
}

impl Probability {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn from_probability_weight(probability_weight: ProbabilityWeight) -> Self {
        Self(probability_weight.into())
    }

    pub fn to_f64(self) -> f64 {
        self.0
    }

    pub fn check_in_bound(&self) -> Result<(), ErrorKind> {
        debug_assert!((0. ..=1.).contains(&self.0));
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum UnitsError {
    #[error("Probability is out of range: {probability:#?}")]
    ProbabilityOutOfRange {
        probability: Probability,
        context: trc,
    },

    #[error("Probability sum is not 1 but {probability_sum:#?}")]
    ProbabilitySumNot1 {
        probability_sum: Probability,
        context: trc,
    },
}

use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use anyhow::{ensure, Ok};
use derive_more::*;

use crate::rules::*;

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
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
        debug_assert!(amount >= 0.);
        Self(amount)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Amount {
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

impl std::fmt::Display for Entropy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

impl Display for Probability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

    pub fn check_in_bound(&self) -> anyhow::Result<()> {
        ensure!(
            (0. ..=1.).contains(&self.0),
            "Probability is out of bound [0, 1] with {}",
            self.0
        );
        Ok(())
    }
}

#[derive(
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    Default,
    Debug,
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
pub struct Time(i64);

impl From<i64> for Time {
    fn from(time: i64) -> Self {
        Self(time)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Time {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn increment_by(&mut self, amount: Amount) {
        self.0 += amount.0 as i64;
    }
}

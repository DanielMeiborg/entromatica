use std::hash::{Hash, Hasher};

use derive_more::*;

use crate::rules::ProbabilityWeight;

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
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
pub struct Amount(pub f64);

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

impl Amount {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
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
pub struct Entropy(pub f64);

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

impl Entropy {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
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
pub struct Probability(pub f64);

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

impl Probability {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }

    pub fn from_probability_weight(probability_weight: ProbabilityWeight) -> Self {
        Self(probability_weight.0)
    }
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
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
pub struct Time(pub i64);

impl Time {
    pub fn new() -> Self {
        Self(0)
    }
}
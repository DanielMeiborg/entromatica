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
        if amount < 0. {
            panic!("Amount cannot be negative");
        }
        Self(amount)
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
    Display,
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
        if entropy < 0. {
            panic!("Entropy cannot be negative");
        }
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
        if !(0. ..=1.).contains(&probability) {
            panic!("Probability must be between 0 and 1");
        }
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
    Display,
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
pub struct Time(i64);

impl From<i64> for Time {
    fn from(time: i64) -> Self {
        if time < 0 {
            panic!("Time cannot be negative");
        }
        Self(time)
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

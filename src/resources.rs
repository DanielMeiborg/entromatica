use derive_more::*;

use crate::units::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct ResourceName(pub String);

impl ResourceName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
pub enum Capacity {
    Limited(Amount),
    Unlimited,
}

impl Capacity {
    pub fn new(limit: Option<Amount>) -> Self {
        match limit {
            Some(amount) => Self::Limited(amount),
            None => Self::Unlimited,
        }
    }
}

/// A resource in the simulation which may or may not have a capacity.
///
/// A resource is essentially a parameter an entity and thus ultimately a state can have.
/// The capacity is a constrain on the amount of the resource being distributed among the entities.
/// It is allowed that the sum of the amounts of a resource among all entities is lesser than the capacity.
/// It is assumed that the capacity is always greater than or equal to zero.
///
/// The capacity_per_entity is an additional constrain on the amount of the resource an individual entity can have.
/// This can again be unlimited.
#[derive(PartialEq, Clone, Debug)]
pub struct Resource {
    pub description: String,
    pub capacity: Capacity,
    pub capacity_per_entity: Capacity,
}

impl Resource {
    pub fn new(description: String, capacity: Capacity, capacity_per_entity: Capacity) -> Self {
        Self {
            description,
            capacity,
            capacity_per_entity,
        }
    }
}

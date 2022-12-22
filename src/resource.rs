use std::fmt::Display;

use derive_more::*;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use crate::error::*;
use crate::state::*;
use crate::units::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, AsRef, AsMut, Into)]
pub struct ResourceName(String);

impl ResourceName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

#[derive(PartialEq, Clone, Debug, Display, From, Default)]
#[allow(dead_code)]
pub enum Capacity {
    Limited(Amount),
    #[default]
    Unlimited,
}

impl Capacity {
    pub fn new() -> Self {
        Self::default()
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
#[derive(PartialEq, Clone, Debug, Default, From, Into)]
pub struct Resource {
    description: String,
    capacity: Capacity,
    capacity_per_entity: Capacity,
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Resource: {}", self.description)?;
        writeln!(f, "Capacity: {:?}", self.capacity)?;
        writeln!(f, "Capacity per entity: {:?}", self.capacity_per_entity)?;
        Ok(())
    }
}

impl Resource {
    pub fn new() -> Self {
        Self {
            description: "".to_string(),
            capacity: Capacity::new(),
            capacity_per_entity: Capacity::new(),
        }
    }

    pub fn from(description: String, capacity: Capacity, capacity_per_entity: Capacity) -> Self {
        Self {
            description,
            capacity,
            capacity_per_entity,
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn capacity(&self) -> &Capacity {
        &self.capacity
    }

    pub fn capacity_per_entity(&self) -> &Capacity {
        &self.capacity_per_entity
    }

    /// Checks if the given state satisfies all resource constrains.
    pub(crate) fn check_resource_capacities(
        resources: &HashMap<ResourceName, Resource>,
        state: &State,
    ) -> Result<(), ErrorKind> {
        for (resource_name, resource) in resources {
            match &resource.capacity {
                Capacity::Limited(limit) => {
                    let mut total_amount = Amount::from(0.);
                    for (_, entity) in state.iter_entities() {
                        let entity_amount = entity.resource(resource_name)?;
                        total_amount += *entity_amount;
                        if total_amount > *limit || total_amount < Amount::from(0.) {
                            return Err(ErrorKind::TotalAmountExceedsResourceLimit(
                                OutOfRangeError::new(total_amount, Amount::from(0.), *limit),
                            ));
                        }
                    }
                }
                Capacity::Unlimited => {
                    for (_, entity) in state.iter_entities() {
                        let entity_amount = entity.resource(resource_name)?;
                        if *entity_amount < Amount::from(0.) {
                            return Err(ErrorKind::AmountIsNegative(OutOfRangeError::new(
                                *entity_amount,
                                Amount::from(0.),
                                Amount::from(f64::INFINITY),
                            )));
                        }
                    }
                }
            }

            match &resource.capacity_per_entity {
                Capacity::Limited(limit) => {
                    for (_, entity) in state.iter_entities() {
                        let entity_amount = entity.resource(resource_name)?;
                        if entity_amount > limit {
                            return Err(ErrorKind::AmountExceedsEntityLimit(OutOfRangeError::new(
                                *entity_amount,
                                Amount::from(0.),
                                *limit,
                            )));
                        }
                    }
                }
                Capacity::Unlimited => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[warn(unused_imports)]
    use super::*;

    #[test]
    fn check_resource_capacities_should_pass_on_maintained_limit() {
        let resources = HashMap::from([(
            ResourceName::from("Gold".to_string()),
            Resource::from(
                "Gold".to_string(),
                Capacity::Limited(Amount::from(10.)),
                Capacity::Unlimited,
            ),
        )]);
        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Gold".to_string()),
                Amount::from(5.),
            )]),
        )]);

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }

    #[test]
    fn check_resource_capacities_should_pass_on_maintained_entity_limit() {
        let resources = HashMap::from([(
            ResourceName::from("Gold".to_string()),
            Resource::from(
                "Gold".to_string(),
                Capacity::Unlimited,
                Capacity::Limited(Amount::from(10.)),
            ),
        )]);

        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Gold".to_string()),
                Amount::from(5.),
            )]),
        )]);

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }

    #[test]
    #[should_panic]
    fn check_resource_capacities_should_panic_on_negative_amounts() {
        let resources = HashMap::from([(
            ResourceName::from("Gold".to_string()),
            Resource::from("Gold".to_string(), Capacity::Unlimited, Capacity::Unlimited),
        )]);

        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Gold".to_string()),
                Amount::from(-1.),
            )]),
        )]);

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }

    #[test]
    #[should_panic]
    fn check_resource_capacities_should_panic_on_exceeded_limit() {
        let resources = HashMap::from([(
            ResourceName::from("Gold".to_string()),
            Resource::from(
                "Gold".to_string(),
                Capacity::Limited(Amount::from(10.)),
                Capacity::Unlimited,
            ),
        )]);

        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Gold".to_string()),
                Amount::from(11.),
            )]),
        )]);

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }

    #[test]
    #[should_panic]
    fn check_resource_capacities_should_panic_on_exceeded_entity_limit() {
        let resources = HashMap::from([(
            ResourceName::from("Gold".to_string()),
            Resource::from(
                "Gold".to_string(),
                Capacity::Unlimited,
                Capacity::Limited(Amount::from(10.)),
            ),
        )]);

        let state = State::from_entities(
            vec![
                (
                    EntityName::from("Someone".to_string()),
                    Entity::from_resources(vec![(
                        ResourceName::from("Gold".to_string()),
                        Amount::from(11.),
                    )]),
                ),
                (
                    EntityName::from("SomeoneElse".to_string()),
                    Entity::from_resources(vec![(
                        ResourceName::from("Gold".to_string()),
                        Amount::from(9.),
                    )]),
                ),
            ]
            .into_iter()
            .collect(),
        );

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }

    #[test]
    #[should_panic]
    fn check_resource_capacities_should_panic_on_nonexisting_resources() {
        let resources = HashMap::from([(
            ResourceName::from("nonexistium".to_string()),
            Resource::new(),
        )]);

        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![]),
        )]);

        Resource::check_resource_capacities(&resources, &state).unwrap();
    }
}

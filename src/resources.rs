use derive_more::*;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

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

    /// Checks if the given state satisfies all resource constrains.
    pub(crate) fn assert_resource_capacities(
        resources: &HashMap<ResourceName, Resource>,
        state: &State,
    ) {
        for (resource_name, resource) in resources {
            match &resource.capacity {
                Capacity::Limited(limit) => {
                    let mut total_amount = Amount::from(0.);
                    for (entity_name, entity) in state.iter_entities() {
                        let entity_amount = entity
                            .resource(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if entity_amount < Amount::from(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                        total_amount += entity_amount;
                        if total_amount > *limit {
                            panic!(
                                "Resource limit exceeded for resource {resource_name}",
                                resource_name = resource_name
                            );
                        }
                    }
                }
                Capacity::Unlimited => {
                    for (entity_name, entity) in state.iter_entities() {
                        let entity_amount = entity
                            .resource(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if entity_amount < Amount::from(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                    }
                }
            }

            match &resource.capacity_per_entity {
                Capacity::Limited(limit) => {
                    for (entity_name, entity) in state.iter_entities() {
                        let entity_amount = entity
                            .resource(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if entity_amount > *limit {
                            panic!(
                                "Entity {} has exceeded resource limit for resource {}",
                                entity_name, resource_name
                            );
                        }
                    }
                }
                Capacity::Unlimited => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[warn(unused_imports)]
    use super::*;

    #[test]
    fn assert_resource_capacities_should_pass_on_maintained_limit() {
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

        Resource::assert_resource_capacities(&resources, &state);
    }

    #[test]
    fn assert_resource_capacities_should_pass_on_maintained_entity_limit() {
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

        Resource::assert_resource_capacities(&resources, &state);
    }

    #[test]
    #[should_panic]
    fn assert_resource_capacities_should_panic_on_negative_amounts() {
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

        Resource::assert_resource_capacities(&resources, &state);
    }

    #[test]
    #[should_panic]
    fn assert_resource_capacities_should_panic_on_exceeded_limit() {
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

        Resource::assert_resource_capacities(&resources, &state);
    }

    #[test]
    #[should_panic]
    fn assert_resource_capacities_should_panic_on_exceeded_entity_limit() {
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

        Resource::assert_resource_capacities(&resources, &state);
    }

    #[test]
    #[should_panic]
    fn assert_resource_capacities_should_panic_on_nonexisting_resources() {
        let resources = HashMap::from([(
            ResourceName::from("nonexistium".to_string()),
            Resource::new(),
        )]);

        let state = State::from_entities(vec![(
            EntityName::from("Someone".to_string()),
            Entity::from_resources(vec![]),
        )]);

        Resource::assert_resource_capacities(&resources, &state);
    }
}

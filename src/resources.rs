use derive_more::*;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use crate::state::*;
use crate::units::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, AsRef, AsMut, Into)]
pub struct ResourceName(pub String);

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
    pub description: String,
    pub capacity: Capacity,
    pub capacity_per_entity: Capacity,
}

impl Resource {
    pub fn new() -> Self {
        Self {
            description: "".to_string(),
            capacity: Capacity::new(),
            capacity_per_entity: Capacity::new(),
        }
    }

    /// Checks if the given state satisfies all resource constrains.
    pub(crate) fn check_resource_capacities(resources: &HashMap<ResourceName, Resource>, state: &State) {
        for (resource_name, resource) in resources {
            match &resource.capacity {
                Capacity::Limited(limit) => {
                    let mut total_amount = Amount(0.);
                    for (entity_name, entity) in &state.entities {
                        let entity_amount = entity
                            .resources
                            .get(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if *entity_amount < Amount(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                        total_amount += *entity_amount;
                        if total_amount > *limit {
                            panic!(
                                "Resource limit exceeded for resource {resource_name}",
                                resource_name = resource_name
                            );
                        }
                    }
                }
                Capacity::Unlimited => {
                    for (entity_name, entity) in &state.entities {
                        let entity_amount = entity
                            .resources
                            .get(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if *entity_amount < Amount(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                    }
                }
            }
        }
    }
}

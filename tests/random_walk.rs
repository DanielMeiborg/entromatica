use entromatica::prelude::*;

use hashbrown::HashMap;
const MAX_ENTITIES: usize = 5;
const MAX_TIME: usize = 10;

fn setup() -> Simulation {
    let resources = HashMap::from([(
        ResourceName::from("Point".to_string()),
        Resource::from(
            "".to_string(),
            Capacity::Limited(Amount::from(1.)),
            Capacity::Limited(Amount::from(1.)),
        ),
    )]);

    let mut state_vec: Vec<(EntityName, Entity)> = (1..MAX_ENTITIES)
        .into_iter()
        .map(|i| {
            (
                EntityName::from(i.to_string()),
                Entity::from_resources(vec![(
                    ResourceName::from("Point".to_string()),
                    Amount::from(0.),
                )]),
            )
        })
        .collect();
    state_vec.push((
        EntityName::from("0".to_string()),
        Entity::from_resources(vec![(
            ResourceName::from("Point".to_string()),
            Amount::from(1.),
        )]),
    ));
    let initial_state = State::from_entities(state_vec);

    let rules = HashMap::from([
        (
            RuleName::from("Move Point to next entity".to_string()),
            Rule::from(
                "".to_string(),
                |_| RuleApplies::from(true),
                ProbabilityWeight::from(1.),
                |state| {
                    let current_entity_name = state
                        .iter_entities()
                        .find(|(_, entity)| {
                            *entity
                                .resource(&ResourceName::from("Point".to_string()))
                                .unwrap()
                                == Amount::from(1.)
                        })
                        .unwrap()
                        .0
                        .clone();

                    let next_entity_name = EntityName::from(
                        {
                            (current_entity_name.to_string().parse::<i64>().unwrap() + 1)
                                .rem_euclid(MAX_ENTITIES as i64)
                        }
                        .to_string(),
                    );

                    HashMap::from([
                        (
                            ActionName::from("Remove point from current entity".to_string()),
                            Action::from(
                                ResourceName::from("Point".to_string()),
                                current_entity_name,
                                Amount::from(0.),
                            ),
                        ),
                        (
                            ActionName::from("Add point to next entity".to_string()),
                            Action::from(
                                ResourceName::from("Point".to_string()),
                                next_entity_name,
                                Amount::from(1.),
                            ),
                        ),
                    ])
                },
            ),
        ),
        (
            RuleName::from("Move Point to previous entity".to_string()),
            Rule::from(
                "".to_string(),
                |_| RuleApplies::from(true),
                ProbabilityWeight::from(1.),
                |state| {
                    let current_entity_name = state
                        .iter_entities()
                        .find(|(_, entity)| {
                            *entity
                                .resource(&ResourceName::from("Point".to_string()))
                                .unwrap()
                                == Amount::from(1.)
                        })
                        .unwrap()
                        .0
                        .clone();

                    let next_entity_name = EntityName::from(
                        {
                            (current_entity_name.to_string().parse::<i64>().unwrap() - 1)
                                .rem_euclid(MAX_ENTITIES as i64)
                        }
                        .to_string(),
                    );

                    HashMap::from([
                        (
                            ActionName::from("Remove point from current entity".to_string()),
                            Action::from(
                                ResourceName::from("Point".to_string()),
                                current_entity_name,
                                Amount::from(0.),
                            ),
                        ),
                        (
                            ActionName::from("Add point to next entity".to_string()),
                            Action::from(
                                ResourceName::from("Point".to_string()),
                                next_entity_name,
                                Amount::from(1.),
                            ),
                        ),
                    ])
                },
            ),
        ),
    ]);

    Simulation::from(resources, initial_state, rules).unwrap()
}

/// This test simulates a one dimensional random walk with connected edges.
#[test]
fn random_walk() {
    let mut simulation = setup();
    assert_eq!(simulation.reachable_states().len(), 1);
    assert_eq!(simulation.entropy(), Entropy::from(0.));
    for _ in 0..MAX_TIME {
        simulation.next_step().unwrap();
    }
    assert_eq!(simulation.reachable_states().len(), MAX_ENTITIES);
    assert_eq!(simulation.entropy(), Entropy::from(2.3009662938553714));
}

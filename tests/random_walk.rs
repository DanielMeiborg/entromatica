use entromatica::prelude::*;

use hashbrown::HashMap;
const MAX_AMOUNT: f64 = 4.;
const MAX_TIME: usize = 10;

fn setup() -> Simulation {
    let initial_state = State::new(vec![(
        EntityName::new("main"),
        Entity::new(vec![(ParameterName::new("point"), Amount::from(0.))]),
    )]);

    let rules = HashMap::from([
        (
            RuleName::new("walk forward"),
            Rule::new(
                "".to_string(),
                Condition::Always,
                ProbabilityWeight::from(1.),
                Action::SetFunction(|state| {
                    let current_point = state
                        .entity(&EntityName::new("main"))
                        .unwrap()
                        .parameter(&ParameterName::new("point"))
                        .unwrap()
                        .to_owned()
                        .to_f64();
                    if current_point < MAX_AMOUNT {
                        HashMap::from([(
                            EntityName::new("main"),
                            (
                                ParameterName::new("point"),
                                Amount::from(current_point + 1.),
                            ),
                        )])
                    } else {
                        HashMap::from([(
                            EntityName::new("main"),
                            (ParameterName::new("point"), Amount::from(0.)),
                        )])
                    }
                }),
            ),
        ),
        (
            RuleName::new("walk back"),
            Rule::new(
                "".to_string(),
                Condition::Always,
                ProbabilityWeight::from(1.),
                Action::SetFunction(|state| {
                    let current_point = state
                        .entity(&EntityName::new("main"))
                        .unwrap()
                        .parameter(&ParameterName::new("point"))
                        .unwrap()
                        .to_owned()
                        .to_f64();
                    if current_point == 0. {
                        HashMap::from([(
                            EntityName::new("main"),
                            (ParameterName::new("point"), Amount::from(MAX_AMOUNT)),
                        )])
                    } else {
                        HashMap::from([(
                            EntityName::new("main"),
                            (
                                ParameterName::new("point"),
                                Amount::from(current_point - 1.),
                            ),
                        )])
                    }
                }),
            ),
        ),
    ]);
    Simulation::new(initial_state, rules)
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
    assert_eq!(simulation.possible_states().clone(), {
        let mut simulation_clone = setup();
        simulation_clone.full_traversal(Some(100), true).unwrap();
        simulation_clone.possible_states().clone()
    });
    assert_eq!(simulation.reachable_states().clone(), {
        let mut simulation_clone = setup();
        for _ in 0..MAX_TIME {
            simulation_clone.next().unwrap().unwrap();
        }
        simulation_clone.reachable_states().clone()
    });
    assert_eq!(simulation.reachable_states().len(), MAX_AMOUNT as usize + 1);
    assert_eq!(simulation.entropy(), Entropy::from(2.3009662938553714));
    let expected_reachable_states = {
        let mut expected_reachable_states = ReachableStates::new();
        let expected_reachable_states_vec: Vec<(u64, f64)> = vec![
            (16318861240434570188, 0.21484375),
            (8001857008351451444, 0.21484375),
            (10921680398206464020, 0.248046875),
            (7911799719936081424, 0.1611328125),
            (3732191206693521782, 0.1611328125),
        ];
        for (state_hash, probability) in expected_reachable_states_vec {
            expected_reachable_states
                .append_state(StateHash::from(state_hash), Probability::from(probability))
                .unwrap();
        }
        expected_reachable_states
    };
    assert_eq!(simulation.reachable_states(), &expected_reachable_states);
    assert!(simulation
        .uniform_distribution_is_steady(Some(100))
        .unwrap(),);
    simulation
        .apply_intervention(&HashMap::from([(
            RuleName::new("Go to 0"),
            Rule::new(
                "Go to 0".to_string(),
                Condition::Always,
                ProbabilityWeight::from(1.),
                Action::SetParameter(
                    EntityName::new("main"),
                    ParameterName::new("point"),
                    Amount::from(0.),
                ),
            ),
        )]))
        .unwrap();
    assert_eq!(simulation.reachable_states().len(), 1);
    assert_eq!(simulation.entropy(), Entropy::from(0.));

    let graph = simulation.graph(Some(100)).unwrap();
    assert_eq!(graph.edge_count(), 15);
    assert_eq!(graph.node_count(), 5);
    assert_eq!(
        simulation
            .history()
            .steps()
            .iter()
            .map(|step| step.reachable_states().entropy().into())
            .collect::<Vec<f64>>(),
        vec![
            0.,
            1.,
            1.5,
            1.811278124459133,
            2.0306390622295662,
            2.135692411043098,
            2.2039336144561052,
            2.2455642866016,
            2.272523933965724,
            2.289757715995956,
            2.3009662938553714,
            0.,
        ]
    );
}

use entromatica::prelude::*;

use hashbrown::HashMap;
const MAX_AMOUNT: i32 = 4;
const MAX_TIME: usize = 10;

fn setup() -> Simulation<i32> {
    let initial_state = State::new(vec![(
        EntityName::new("main"),
        Entity::new(vec![(ParameterName::new("point"), Parameter::new(0))]),
    )]);

    let rules = HashMap::from([
        (
            RuleName::new("walk forward"),
            Rule::new(
                "".to_string(),
                Condition::Always,
                ProbabilityWeight::from(1.),
                Action::SetFunction(|state: State<i32>| {
                    let current_point = *state
                        .entity(&EntityName::new("main"))
                        .unwrap()
                        .parameter(&ParameterName::new("point"))
                        .unwrap()
                        .to_owned()
                        .value();
                    if current_point < MAX_AMOUNT {
                        HashMap::from([(
                            EntityName::new("main"),
                            (
                                ParameterName::new("point"),
                                Parameter::new(current_point + 1),
                            ),
                        )])
                    } else {
                        HashMap::from([(
                            EntityName::new("main"),
                            (ParameterName::new("point"), Parameter::new(0)),
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
                    let current_point = *state
                        .entity(&EntityName::new("main"))
                        .unwrap()
                        .parameter(&ParameterName::new("point"))
                        .unwrap()
                        .to_owned()
                        .value();
                    if current_point == 0 {
                        HashMap::from([(
                            EntityName::new("main"),
                            (ParameterName::new("point"), Parameter::new(MAX_AMOUNT)),
                        )])
                    } else {
                        HashMap::from([(
                            EntityName::new("main"),
                            (
                                ParameterName::new("point"),
                                Parameter::new(current_point - 1),
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
            (1983155860954835072, 0.21484375),
            (4317319517181003110, 0.21484375),
            (6119077233511131479, 0.248046875),
            (15201673400525796111, 0.1611328125),
            (17580441199265992620, 0.1611328125),
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
                    Parameter::new(0),
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
    assert_eq!(
        simulation
            .history()
            .steps()
            .iter()
            .map(|step| step.reachable_states().len())
            .collect::<Vec<usize>>(),
        vec![1, 2, 3, 4, 5, 5, 5, 5, 5, 5, 5, 1]
    );
}

#[test]
fn serialization() {
    let mut simulation = setup();
    for _ in 0..MAX_TIME {
        simulation.next_step().unwrap();
    }
    let serialized_reachable_states = serde_json::to_string(simulation.reachable_states()).unwrap();
    assert_eq!(
        serialized_reachable_states,
        r#"{"6119077233511131479":0.248046875,"15201673400525796111":0.1611328125,"1983155860954835072":0.21484375,"17580441199265992620":0.1611328125,"4317319517181003110":0.21484375}"#
    );

    let serialized_possible_states = serde_json::to_string(simulation.possible_states()).unwrap();
    assert_eq!(
        serialized_possible_states,
        r#"{"6119077233511131479":{"entities":{"main":{"parameters":{"point":{"value":0}}}}},"15201673400525796111":{"entities":{"main":{"parameters":{"point":{"value":1}}}}},"1983155860954835072":{"entities":{"main":{"parameters":{"point":{"value":2}}}}},"17580441199265992620":{"entities":{"main":{"parameters":{"point":{"value":4}}}}},"4317319517181003110":{"entities":{"main":{"parameters":{"point":{"value":3}}}}}}"#
    );
    let serializable_simulation = simulation.to_serializable();
    let simulation_string = serde_json::to_string(&serializable_simulation).unwrap();
    assert_eq!(
        simulation_string,
        r#"{"history":{"steps":[{"reachable_states":{"6119077233511131479":1.0},"applied_rules":[]},{"reachable_states":{"17580441199265992620":0.5,"15201673400525796111":0.5},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"1983155860954835072":0.25,"6119077233511131479":0.5,"4317319517181003110":0.25},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"4317319517181003110":0.125,"15201673400525796111":0.375,"17580441199265992620":0.375,"1983155860954835072":0.125},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.375,"4317319517181003110":0.25,"1983155860954835072":0.25,"17580441199265992620":0.0625,"15201673400525796111":0.0625},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.0625,"15201673400525796111":0.3125,"1983155860954835072":0.15625,"17580441199265992620":0.3125,"4317319517181003110":0.15625},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.3125,"15201673400525796111":0.109375,"1983155860954835072":0.234375,"17580441199265992620":0.109375,"4317319517181003110":0.234375},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.109375,"15201673400525796111":0.2734375,"1983155860954835072":0.171875,"17580441199265992620":0.2734375,"4317319517181003110":0.171875},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.2734375,"15201673400525796111":0.140625,"1983155860954835072":0.22265625,"17580441199265992620":0.140625,"4317319517181003110":0.22265625},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.140625,"15201673400525796111":0.248046875,"1983155860954835072":0.181640625,"17580441199265992620":0.248046875,"4317319517181003110":0.181640625},"applied_rules":["walk forward","walk back"]},{"reachable_states":{"6119077233511131479":0.248046875,"15201673400525796111":0.1611328125,"1983155860954835072":0.21484375,"17580441199265992620":0.1611328125,"4317319517181003110":0.21484375},"applied_rules":["walk forward","walk back"]}]},"rules":["walk forward","walk back"],"possible_states":{"6119077233511131479":{"entities":{"main":{"parameters":{"point":{"value":0}}}}},"15201673400525796111":{"entities":{"main":{"parameters":{"point":{"value":1}}}}},"1983155860954835072":{"entities":{"main":{"parameters":{"point":{"value":2}}}}},"17580441199265992620":{"entities":{"main":{"parameters":{"point":{"value":4}}}}},"4317319517181003110":{"entities":{"main":{"parameters":{"point":{"value":3}}}}}},"cache":{"rules":{"walk forward":{"condition":{"6119077233511131479":true,"15201673400525796111":true,"1983155860954835072":true,"17580441199265992620":true,"4317319517181003110":true},"actions":{"6119077233511131479":15201673400525796111,"15201673400525796111":1983155860954835072,"1983155860954835072":4317319517181003110,"17580441199265992620":6119077233511131479,"4317319517181003110":17580441199265992620}},"walk back":{"condition":{"6119077233511131479":true,"15201673400525796111":true,"1983155860954835072":true,"17580441199265992620":true,"4317319517181003110":true},"actions":{"6119077233511131479":17580441199265992620,"15201673400525796111":6119077233511131479,"1983155860954835072":15201673400525796111,"17580441199265992620":4317319517181003110,"4317319517181003110":1983155860954835072}}}}}"#
    );
    assert_eq!(
        serde_json::from_value::<SerializableSimulation<i32>>(
            serde_json::to_value(&serializable_simulation).unwrap()
        )
        .unwrap(),
        serializable_simulation
    );
    let reconstructed_simulation: Simulation<i32> =
        Simulation::from_serializable(serializable_simulation, simulation.rules().clone()).unwrap();
    assert_eq!(reconstructed_simulation, simulation);
}

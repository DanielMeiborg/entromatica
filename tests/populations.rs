use std::sync::Arc;

use entromatica::prelude::*;
use hashbrown::HashMap;

const PREDATOR_FACTOR: f64 = 0.2;
const PREDATOR_COST: i32 = 10;
const PREY_PRODUCTION: i32 = 100;

#[test]
fn predator_prey() {
    let initial_state = State::new(vec![
        (
            EntityName::new("Predators"),
            Entity::new(vec![(
                ParameterName::new("Population"),
                Parameter::new(100),
            )]),
        ),
        (
            EntityName::new("Prey"),
            Entity::new(vec![(
                ParameterName::new("Population"),
                Parameter::new(100),
            )]),
        ),
    ]);
    let predator_rule = EntityRule::new(
        "predator rule".to_string(),
        Arc::new(|_| RuleApplies::new(true)),
        ProbabilityWeight::from(0.5),
        Arc::new(|state: State<i32>| {
            let mut new_entity: Entity<i32> =
                state.entity(&EntityName::new("Predators")).unwrap().clone();
            let parameter: &mut Parameter<i32> = new_entity
                .parameter_mut(&ParameterName::new("Population"))
                .unwrap();
            *parameter = Parameter::new(
                parameter.value()
                    + state
                        .entity(&EntityName::new("Prey"))
                        .unwrap()
                        .parameter(&ParameterName::new("Population"))
                        .unwrap()
                        .value()
                        * PREDATOR_FACTOR as i32
                    - PREDATOR_COST,
            );
            new_entity
        }),
    );
    let prey_rule = EntityRule::new(
        "prey rule".to_string(),
        Arc::new(|_| RuleApplies::new(true)),
        ProbabilityWeight::from(0.5),
        Arc::new(|state: State<i32>| {
            let mut new_entity: Entity<i32> =
                state.entity(&EntityName::new("Prey")).unwrap().clone();
            let parameter: &mut Parameter<i32> = new_entity
                .parameter_mut(&ParameterName::new("Population"))
                .unwrap();
            *parameter = Parameter::new(
                parameter.value()
                    - (*state
                        .entity(&EntityName::new("Predators"))
                        .unwrap()
                        .parameter(&ParameterName::new("Population"))
                        .unwrap()
                        .value() as f64
                        * PREDATOR_FACTOR) as i32
                    + PREY_PRODUCTION,
            );
            new_entity
        }),
    );
    let rules = combine_entity_rules(HashMap::from([
        (EntityName::new("Predators"), predator_rule),
        (EntityName::new("Prey"), prey_rule),
    ]));
    assert_eq!(rules.len(), 4);
    let mut simulation = Simulation::new(initial_state, rules);
    simulation.run(3).unwrap();
    println!("Finished running the simulation");
    let serialized_reachable_states = serde_json::to_value(simulation.reachable_states()).unwrap();
    println!("{serialized_reachable_states}");
    let expected_reachable_states = r#"{"15947393681996075094":0.008360993676739883,"17668138504944923618":0.008360993676739883,"6452912304633146823":0.008360993676739883,"103691826577321544":0.008360993676739883,"4898386825796708457":0.05729303823330874,"3763009900284740360":0.06459714043141399,"16783279961287950349":0.008360993676739883,"11279678175951637949":0.0562361467546741,"15778416642766731576":0.008360993676739883,"1935527284584074281":0.008360993676739883,"2472675042169194355":0.0562361467546741,"6914758398991409719":0.05729303823330874,"17226391052054714035":0.048932044556568854,"492955319960924360":0.008360993676739883,"10543129331064510733":0.008360993676739883,"17562471794795536369":0.008360993676739883,"2035503167842838108":0.048932044556568854,"7297238898932148814":0.008360993676739883,"7090304239799839397":0.07508468627929688,"17030601309466462408":0.008360993676739883,"6017207179085392128":0.008360993676739883,"5032165958533093496":0.016721987353479767,"13842768791620211793":0.008360993676739883,"921366409929830603":0.008360993676739883,"10434612972465901004":0.10345276189036667,"8064893785939925835":0.008360993676739883,"1794209793831128195":0.008360993676739883,"654152135830653267":0.008360993676739883,"16765659541721254555":0.008360993676739883,"12857151577878363143":0.10345276189036667,"9861348979414438548":0.048932044556568854,"13923961920932472213":0.15238480644693553}"#;
    println!("{expected_reachable_states}");
    assert_eq!(
        serde_json::from_str::<HashMap<String, f64>>(expected_reachable_states).unwrap(),
        serde_json::from_value(serialized_reachable_states).unwrap()
    );
}

// Safety invariants and constraint helpers.
// Currently the main enforcement lives in L0 executor and L1 sandbox.

pub fn default_constraints() -> std::collections::HashMap<String, f64> {
    let mut c = std::collections::HashMap::new();
    c.insert("max_risk".into(), 0.65);
    c.insert("max_actions_per_step".into(), 3.0);
    c
}

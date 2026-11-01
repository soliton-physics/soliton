/// A conserved quantity evaluated on the current state.
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    pub name: &'static str,
    pub value: f64,
}

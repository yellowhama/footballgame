#[derive(Debug)]
pub enum HardGateResult {
    Pass,
    Fail,
}

impl HardGateResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Pass)
    }
}

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum BudgetKind {
    PossessionOutcome,
    ShotOutcome,
    TackleOutcome,
}

#[derive(Clone, Debug)]
pub struct BudgetGate {
    used: HashMap<BudgetKind, u32>,
    limit: HashMap<BudgetKind, u32>,
}

impl BudgetGate {
    pub fn new_default() -> Self {
        let mut limit = HashMap::new();
        // 기본 상한: tick/minute 설계에 맞춰 조절
        limit.insert(BudgetKind::PossessionOutcome, 999999);
        limit.insert(BudgetKind::ShotOutcome, 30); // 한 경기 슛 outcome 과다 방지
        limit.insert(BudgetKind::TackleOutcome, 80); // 태클 outcome 과다 방지
        Self { used: HashMap::new(), limit }
    }

    pub fn can_spend(&self, kind: BudgetKind, amount: u32) -> bool {
        let u = *self.used.get(&kind).unwrap_or(&0);
        let lim = *self.limit.get(&kind).unwrap_or(&u32::MAX);
        u.saturating_add(amount) <= lim
    }

    pub fn consume(&mut self, kind: BudgetKind, amount: u32) {
        let u = self.used.entry(kind).or_insert(0);
        *u = u.saturating_add(amount);
    }
}

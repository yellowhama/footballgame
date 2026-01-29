pub mod budget;
pub mod gate;
pub mod softmax;

pub const W_MIN: f32 = 0.05;
pub const W_MAX: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
pub struct WeightBreakdown {
    pub base: f32,
    pub attr: f32,
    pub tactics: f32,
    pub personality: f32,
    pub cards: f32,
    pub context: f32,
}

impl Default for WeightBreakdown {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightBreakdown {
    pub fn new() -> Self {
        Self { base: 1.0, attr: 1.0, tactics: 1.0, personality: 1.0, cards: 1.0, context: 1.0 }
    }

    pub fn neutral() -> Self {
        Self::new()
    }

    pub fn clamp_all(mut self) -> Self {
        self.base = self.base.clamp(0.01, 100.0);
        self.attr = self.attr.clamp(0.01, 100.0);
        self.tactics = self.tactics.clamp(0.01, 100.0);
        self.personality = self.personality.clamp(0.01, 100.0);
        self.cards = self.cards.clamp(0.01, 100.0);
        self.context = self.context.clamp(0.01, 100.0);
        self
    }

    pub fn ln_sum(&self) -> f32 {
        // ln(0) 방지: 최소값 clamp
        let b = self.base.max(0.0001).ln();
        let a = self.attr.max(0.0001).ln();
        let t = self.tactics.max(0.0001).ln();
        let p = self.personality.max(0.0001).ln();
        let c = self.cards.max(0.0001).ln();
        let x = self.context.max(0.0001).ln();
        b + a + t + p + c + x
    }

    pub fn to_weight(&self) -> f32 {
        self.ln_sum().exp().clamp(W_MIN, W_MAX)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StackRule {
    AddLn,
    MaxOnly,
    MinOnly,
}

#[derive(Clone, Debug)]
pub struct Factor {
    pub key: &'static str,
    pub f: f32,
    pub stack: StackRule,
    pub lane: Lane,
}

#[derive(Clone, Copy, Debug)]
pub enum Lane {
    Attr,
    Tactics,
    Personality,
    Cards,
    Context,
}

#[derive(Clone, Debug)]
pub struct WeightComposer {
    factors: Vec<Factor>,
}

impl Default for WeightComposer {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightComposer {
    pub fn new() -> Self {
        Self { factors: Vec::new() }
    }

    pub fn push(&mut self, factor: Factor) {
        self.factors.push(factor);
    }

    // Legacy helper for backward compatibility
    pub fn add(&mut self, key: &'static str, f: f32, stack: StackRule) {
        // Default lane to Context if not specified (legacy behavior)
        self.push(Factor { key, f, stack, lane: Lane::Context });
    }

    // Legacy helper
    pub fn compose(&self) -> f32 {
        self.breakdown_for("legacy").to_weight()
    }

    /// scenario_key 예: "shot.goal", "possess.offside" 등
    pub fn breakdown_for(&self, _scenario_key: &str) -> WeightBreakdown {
        let mut bd = WeightBreakdown::new();

        let mut attr = 1.0;
        let mut tactics = 1.0;
        let mut personality = 1.0;
        let mut cards = 1.0;
        let mut context = 1.0;

        let mut attr_max: f32 = 1.0;
        let mut tactics_max: f32 = 1.0;
        let mut cards_max: f32 = 1.0;
        let mut personality_max: f32 = 1.0;
        let mut context_max: f32 = 1.0;

        let mut attr_min: f32 = 1.0;
        let mut tactics_min: f32 = 1.0;
        let mut cards_min: f32 = 1.0;
        let mut personality_min: f32 = 1.0;
        let mut context_min: f32 = 1.0;

        for f in &self.factors {
            let v = f.f.clamp(0.5, 2.0);
            match (f.lane, f.stack) {
                (Lane::Attr, StackRule::AddLn) => attr *= v,
                (Lane::Tactics, StackRule::AddLn) => tactics *= v,
                (Lane::Personality, StackRule::AddLn) => personality *= v,
                (Lane::Cards, StackRule::AddLn) => cards *= v,
                (Lane::Context, StackRule::AddLn) => context *= v,

                (Lane::Attr, StackRule::MaxOnly) => attr_max = attr_max.max(v),
                (Lane::Tactics, StackRule::MaxOnly) => tactics_max = tactics_max.max(v),
                (Lane::Personality, StackRule::MaxOnly) => personality_max = personality_max.max(v),
                (Lane::Cards, StackRule::MaxOnly) => cards_max = cards_max.max(v),
                (Lane::Context, StackRule::MaxOnly) => context_max = context_max.max(v),

                (Lane::Attr, StackRule::MinOnly) => attr_min = attr_min.min(v),
                (Lane::Tactics, StackRule::MinOnly) => tactics_min = tactics_min.min(v),
                (Lane::Personality, StackRule::MinOnly) => personality_min = personality_min.min(v),
                (Lane::Cards, StackRule::MinOnly) => cards_min = cards_min.min(v),
                (Lane::Context, StackRule::MinOnly) => context_min = context_min.min(v),
            }
        }

        bd.attr = (attr * attr_max * attr_min).clamp(0.5, 2.0);
        bd.tactics = (tactics * tactics_max * tactics_min).clamp(0.5, 2.0);
        bd.personality = (personality * personality_max * personality_min).clamp(0.5, 2.0);
        bd.cards = (cards * cards_max * cards_min).clamp(0.5, 2.0);
        bd.context = (context * context_max * context_min).clamp(0.5, 2.0);
        bd
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TemperatureModel {
    pub base: f32,
}

impl TemperatureModel {
    pub fn from_context(
        ctx: &crate::engine::match_sim::decision_topology::DecisionContext,
    ) -> Self {
        // pressure/fatigue가 높을수록 다양성 증가(T↑)
        let p = ctx.local_pressure.clamp(0.0, 1.0);
        // let f = ctx.fatigue.clamp(0.0, 1.0); // Fatigue not directly in DecisionContext yet
        let t = 1.0 + 0.8 * p; // + 0.6*f;
        Self { base: t.clamp(0.8, 2.5) }
    }

    pub fn temperature_for_set(
        &self,
        _set: crate::engine::match_sim::decision_topology::OutcomeSetId,
    ) -> f32 {
        self.base
    }
}

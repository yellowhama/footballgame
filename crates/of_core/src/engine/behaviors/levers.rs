/// Levers derived from TeamInstructions and DeckImpact
#[derive(Debug, Clone, Copy)]
pub struct BehaviorLevers {
    /// Distance from opponent to stick to when marking (meters)
    pub marking_distance: f32,
    /// Interpolation factor (0.0=loose, 1.0=glue) for marking velocity
    pub marking_stickiness: f32,
    /// Distance from ball to trigger pressing state (meters)
    pub press_trigger_distance: f32,
    /// Distance from ball to give up pressing (meters)
    pub press_giveup_distance: f32,
    /// Stamina multiplier (1.0 = normal, <1.0 = slower drain)
    pub stamina_drain_multiplier: f32,
    /// Tackle success bonus (0.0 = none) - used for visual confidence or future actions
    pub tackle_success_bonus: f32,
    /// Minimum time (ticks) to stay in press before giving up? (optional)
    pub max_press_ticks: u16,
    /// Distance from goal/defender to trigger a forward run
    pub run_trigger_distance: f32,
}

impl Default for BehaviorLevers {
    fn default() -> Self {
        Self {
            marking_distance: 1.0,
            marking_stickiness: 0.5,
            press_trigger_distance: 15.0,
            press_giveup_distance: 22.5,
            stamina_drain_multiplier: 1.0,
            tackle_success_bonus: 0.0,
            max_press_ticks: 60,
            run_trigger_distance: 35.0,
        }
    }
}

//! Sticky action toggles (sprint/dribble/press).

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickyActions {
    pub sprint: bool,
    pub dribble: bool,
    pub press: bool,
}

impl StickyActions {
    pub fn set(&mut self, action: StickyAction, enabled: bool) {
        match action {
            StickyAction::Sprint => self.sprint = enabled,
            StickyAction::Dribble => self.dribble = enabled,
            StickyAction::Press => self.press = enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickyAction {
    Sprint,
    Dribble,
    Press,
}

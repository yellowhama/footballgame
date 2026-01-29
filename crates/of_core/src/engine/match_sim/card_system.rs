//! Card tracking with ejection support (yellow accumulation + red cards).

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    Yellow,
    Red,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardResult {
    Warning,
    Ejection,
}

#[derive(Debug, Default)]
pub struct CardSystem {
    yellow_cards: HashMap<usize, u8>,
    ejected_players: HashSet<usize>,
}

impl CardSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue_card(&mut self, player_idx: usize, card_type: CardType) -> (CardResult, u8) {
        match card_type {
            CardType::Yellow => {
                let count = {
                    let entry = self.yellow_cards.entry(player_idx).or_insert(0);
                    *entry = entry.saturating_add(1);
                    *entry
                };
                if count >= 2 {
                    self.eject_player(player_idx);
                    (CardResult::Ejection, count)
                } else {
                    (CardResult::Warning, count)
                }
            }
            CardType::Red => {
                self.eject_player(player_idx);
                (CardResult::Ejection, 0)
            }
        }
    }

    pub fn is_ejected(&self, player_idx: usize) -> bool {
        self.ejected_players.contains(&player_idx)
    }

    pub fn yellow_count(&self, player_idx: usize) -> u8 {
        *self.yellow_cards.get(&player_idx).unwrap_or(&0)
    }

    pub fn ejected_players(&self) -> impl Iterator<Item = usize> + '_ {
        self.ejected_players.iter().copied()
    }

    pub fn reset(&mut self) {
        self.yellow_cards.clear();
        self.ejected_players.clear();
    }

    fn eject_player(&mut self, player_idx: usize) {
        self.ejected_players.insert(player_idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yellow_then_ejection() {
        let mut cards = CardSystem::new();
        let (result, count) = cards.issue_card(7, CardType::Yellow);
        assert_eq!(result, CardResult::Warning);
        assert_eq!(count, 1);
        assert!(!cards.is_ejected(7));

        let (result, count) = cards.issue_card(7, CardType::Yellow);
        assert_eq!(result, CardResult::Ejection);
        assert_eq!(count, 2);
        assert!(cards.is_ejected(7));
    }

    #[test]
    fn test_direct_red() {
        let mut cards = CardSystem::new();
        let (result, count) = cards.issue_card(3, CardType::Red);
        assert_eq!(result, CardResult::Ejection);
        assert_eq!(count, 0);
        assert!(cards.is_ejected(3));
    }

    #[test]
    fn test_reset_clears() {
        let mut cards = CardSystem::new();
        cards.issue_card(5, CardType::Yellow);
        cards.issue_card(5, CardType::Red);
        cards.reset();
        assert_eq!(cards.yellow_count(5), 0);
        assert!(!cards.is_ejected(5));
    }
}

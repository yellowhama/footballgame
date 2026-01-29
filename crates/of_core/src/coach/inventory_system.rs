// 카드 종류별 독립 인벤토리 시스템
use super::card::{CardType, CoachCard};
use super::tactics::TacticsCard;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manager 전용 인벤토리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerInventory {
    /// 보유한 Manager 카드들
    pub cards: HashMap<String, Vec<CoachCard>>,
    /// 컬렉션 (도감)
    pub collection: Vec<String>,
    /// 최대 용량
    pub max_capacity: usize,
    /// 현재 보유 수
    pub count: usize,
}

/// Coach 전용 인벤토리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachInventory {
    /// 보유한 Coach 카드들
    pub cards: HashMap<String, Vec<CoachCard>>,
    /// 컬렉션 (도감)
    pub collection: Vec<String>,
    /// 최대 용량
    pub max_capacity: usize,
    /// 현재 보유 수
    pub count: usize,
}

/// Tactics 전용 인벤토리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticsInventory {
    /// 보유한 Tactics 카드들
    pub cards: HashMap<String, Vec<TacticsCard>>,
    /// 컬렉션 (도감)
    pub collection: Vec<String>,
    /// 최대 용량
    pub max_capacity: usize,
    /// 현재 보유 수
    pub count: usize,
}

/// Manager 전용 덱 (감독 1장만)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerDeck {
    pub name: String,
    pub manager_card: Option<CoachCard>,
}

/// Coach 전용 덱 (코치 3장)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachDeck {
    pub name: String,
    pub coach_cards: [Option<CoachCard>; 3],
}

/// Tactics 전용 덱 (전술 3장)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticsDeck {
    pub name: String,
    pub tactics_cards: [Option<TacticsCard>; 3],
}

/// 통합 덱 시스템 (각 종류별 덱 조합)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedDeck {
    pub name: String,
    pub manager_deck: ManagerDeck,
    pub coach_deck: CoachDeck,
    pub tactics_deck: TacticsDeck,
    pub total_bonus: f32,
}

// ===== Manager Inventory 구현 =====
impl Default for ManagerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagerInventory {
    pub fn new() -> Self {
        Self { cards: HashMap::new(), collection: Vec::new(), max_capacity: 100, count: 0 }
    }

    pub fn add_card(&mut self, card: CoachCard) -> Result<bool, String> {
        if card.card_type != CardType::Manager {
            return Err("Manager 인벤토리에는 Manager 카드만 추가 가능합니다.".to_string());
        }

        if self.count >= self.max_capacity {
            return Err("Manager 인벤토리가 가득 찼습니다.".to_string());
        }

        let is_new = !self.collection.contains(&card.id);
        if is_new {
            self.collection.push(card.id.clone());
        }

        self.cards.entry(card.id.clone()).or_default().push(card);

        self.count += 1;
        Ok(is_new)
    }

    pub fn get_card(&self, card_id: &str) -> Option<&CoachCard> {
        self.cards.get(card_id).and_then(|cards| cards.first())
    }

    pub fn get_all_cards(&self) -> Vec<CoachCard> {
        self.cards.values().flat_map(|cards| cards.iter().cloned()).collect()
    }
}

// ===== Coach Inventory 구현 =====
impl Default for CoachInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl CoachInventory {
    pub fn new() -> Self {
        Self { cards: HashMap::new(), collection: Vec::new(), max_capacity: 300, count: 0 }
    }

    pub fn add_card(&mut self, card: CoachCard) -> Result<bool, String> {
        if card.card_type != CardType::Coach {
            return Err("Coach 인벤토리에는 Coach 카드만 추가 가능합니다.".to_string());
        }

        if self.count >= self.max_capacity {
            return Err("Coach 인벤토리가 가득 찼습니다.".to_string());
        }

        let is_new = !self.collection.contains(&card.id);
        if is_new {
            self.collection.push(card.id.clone());
        }

        self.cards.entry(card.id.clone()).or_default().push(card);

        self.count += 1;
        Ok(is_new)
    }

    pub fn get_card(&self, card_id: &str) -> Option<&CoachCard> {
        self.cards.get(card_id).and_then(|cards| cards.first())
    }

    pub fn get_all_cards(&self) -> Vec<CoachCard> {
        self.cards.values().flat_map(|cards| cards.iter().cloned()).collect()
    }
}

// ===== Tactics Inventory 구현 =====
impl Default for TacticsInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl TacticsInventory {
    pub fn new() -> Self {
        Self { cards: HashMap::new(), collection: Vec::new(), max_capacity: 200, count: 0 }
    }

    pub fn add_card(&mut self, card: TacticsCard) -> Result<bool, String> {
        if self.count >= self.max_capacity {
            return Err("Tactics 인벤토리가 가득 찼습니다.".to_string());
        }

        let is_new = !self.collection.contains(&card.id);
        if is_new {
            self.collection.push(card.id.clone());
        }

        self.cards.entry(card.id.clone()).or_default().push(card);

        self.count += 1;
        Ok(is_new)
    }

    pub fn get_card(&self, card_id: &str) -> Option<&TacticsCard> {
        self.cards.get(card_id).and_then(|cards| cards.first())
    }

    pub fn get_all_cards(&self) -> Vec<TacticsCard> {
        self.cards.values().flat_map(|cards| cards.iter().cloned()).collect()
    }
}

// ===== Manager Deck 구현 =====
impl ManagerDeck {
    pub fn new(name: String) -> Self {
        Self { name, manager_card: None }
    }

    pub fn set_manager(&mut self, card: CoachCard) -> Result<(), String> {
        if card.card_type != CardType::Manager {
            return Err("Manager 덱에는 Manager 카드만 설정 가능합니다.".to_string());
        }
        self.manager_card = Some(card);
        Ok(())
    }

    pub fn remove_manager(&mut self) -> Option<CoachCard> {
        self.manager_card.take()
    }

    pub fn get_bonus(&self) -> f32 {
        self.manager_card.as_ref().map(|card| card.current_bonus()).unwrap_or(1.0)
    }
}

// ===== Coach Deck 구현 =====
impl CoachDeck {
    pub fn new(name: String) -> Self {
        Self { name, coach_cards: [None, None, None] }
    }

    pub fn set_coach(&mut self, slot: usize, card: CoachCard) -> Result<(), String> {
        if slot >= 3 {
            return Err("Coach 슬롯은 0-2까지입니다.".to_string());
        }
        if card.card_type != CardType::Coach {
            return Err("Coach 덱에는 Coach 카드만 설정 가능합니다.".to_string());
        }
        self.coach_cards[slot] = Some(card);
        Ok(())
    }

    pub fn remove_coach(&mut self, slot: usize) -> Option<CoachCard> {
        if slot < 3 {
            self.coach_cards[slot].take()
        } else {
            None
        }
    }

    pub fn get_total_bonus(&self) -> f32 {
        self.coach_cards
            .iter()
            .filter_map(|card| card.as_ref())
            .map(|card| card.current_bonus())
            .fold(1.0, |acc, bonus| acc * bonus)
    }
}

// ===== Tactics Deck 구현 =====
impl TacticsDeck {
    pub fn new(name: String) -> Self {
        Self { name, tactics_cards: [None, None, None] }
    }

    pub fn set_tactics(&mut self, slot: usize, card: TacticsCard) -> Result<(), String> {
        if slot >= 3 {
            return Err("Tactics 슬롯은 0-2까지입니다.".to_string());
        }
        self.tactics_cards[slot] = Some(card);
        Ok(())
    }

    pub fn remove_tactics(&mut self, slot: usize) -> Option<TacticsCard> {
        if slot < 3 {
            self.tactics_cards[slot].take()
        } else {
            None
        }
    }

    pub fn get_total_bonus(&self) -> f32 {
        self.tactics_cards
            .iter()
            .filter_map(|card| card.as_ref())
            .map(|card| card.current_bonus())
            .fold(1.0, |acc, bonus| acc * bonus)
    }

    pub fn get_active_tactics(&self) -> Vec<super::tactics::TacticalStyle> {
        self.tactics_cards
            .iter()
            .filter_map(|card| card.as_ref().map(|c| c.tactical_style))
            .collect()
    }
}

// ===== Combined Deck 구현 =====
impl CombinedDeck {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            manager_deck: ManagerDeck::new(format!("{}_manager", name)),
            coach_deck: CoachDeck::new(format!("{}_coach", name)),
            tactics_deck: TacticsDeck::new(format!("{}_tactics", name)),
            total_bonus: 1.0,
        }
    }

    pub fn calculate_total_bonus(&mut self) -> f32 {
        let manager_bonus = self.manager_deck.get_bonus();
        let coach_bonus = self.coach_deck.get_total_bonus();
        let tactics_bonus = self.tactics_deck.get_total_bonus();

        // 전술 콤보 체크
        let combo_bonus = self.calculate_combo_bonus();

        self.total_bonus = manager_bonus * coach_bonus * tactics_bonus * combo_bonus;
        self.total_bonus
    }

    fn calculate_combo_bonus(&self) -> f32 {
        let active_tactics = self.tactics_deck.get_active_tactics();
        let combos = super::tactics::get_predefined_combos();

        let mut bonus = 1.0;
        for combo in combos {
            if combo.is_active(&active_tactics) {
                bonus *= 1.0 + combo.bonus_value;
            }
        }
        bonus
    }

    pub fn is_complete(&self) -> bool {
        self.manager_deck.manager_card.is_some()
            && self.coach_deck.coach_cards.iter().all(|c| c.is_some())
            && self.tactics_deck.tactics_cards.iter().all(|t| t.is_some())
    }

    pub fn get_summary(&self) -> String {
        format!(
            "덱: {}\n감독: {}\n코치: {} 명\n전술: {} 개\n총 보너스: x{:.2}",
            self.name,
            if self.manager_deck.manager_card.is_some() { "있음" } else { "없음" },
            self.coach_deck.coach_cards.iter().filter(|c| c.is_some()).count(),
            self.tactics_deck.tactics_cards.iter().filter(|t| t.is_some()).count(),
            self.total_bonus
        )
    }
}

/// 전체 인벤토리 시스템 관리자
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryManager {
    pub manager_inventory: ManagerInventory,
    pub coach_inventory: CoachInventory,
    pub tactics_inventory: TacticsInventory,
    pub combined_decks: Vec<CombinedDeck>,
    pub active_deck_index: Option<usize>,
}

impl Default for InventoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryManager {
    pub fn new() -> Self {
        Self {
            manager_inventory: ManagerInventory::new(),
            coach_inventory: CoachInventory::new(),
            tactics_inventory: TacticsInventory::new(),
            combined_decks: vec![CombinedDeck::new("기본 덱".to_string())],
            active_deck_index: Some(0),
        }
    }

    pub fn add_deck(&mut self, name: String) -> Result<(), String> {
        if self.combined_decks.len() >= 5 {
            return Err("최대 5개의 덱만 보유 가능합니다.".to_string());
        }
        self.combined_decks.push(CombinedDeck::new(name));
        Ok(())
    }

    pub fn get_active_deck(&self) -> Option<&CombinedDeck> {
        self.active_deck_index.and_then(|idx| self.combined_decks.get(idx))
    }

    pub fn get_active_deck_mut(&mut self) -> Option<&mut CombinedDeck> {
        self.active_deck_index.and_then(move |idx| self.combined_decks.get_mut(idx))
    }

    pub fn get_total_summary(&self) -> String {
        format!(
            "===== 인벤토리 현황 =====\n\
            Manager 카드: {}/{}\n\
            Coach 카드: {}/{}\n\
            Tactics 카드: {}/{}\n\
            \n===== 덱 현황 =====\n\
            보유 덱: {}개\n\
            활성 덱: {}",
            self.manager_inventory.count,
            self.manager_inventory.max_capacity,
            self.coach_inventory.count,
            self.coach_inventory.max_capacity,
            self.tactics_inventory.count,
            self.tactics_inventory.max_capacity,
            self.combined_decks.len(),
            self.active_deck_index.map_or("없음".to_string(), |i| format!("{}", i + 1))
        )
    }
}

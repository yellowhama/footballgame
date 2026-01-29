// 통합 인벤토리 시스템 - 3가지 카드 타입별 관리
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::card::{CoachCard, CardRarity, CardType, Specialty};
use super::tactics::{TacticsCard, TacticalStyle};
use super::gacha::GachaCard;
use super::deck::Deck;

/// 통합 카드 인벤토리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedInventory {
    /// Manager 카드 보관함
    pub manager_cards: HashMap<String, Vec<CoachCard>>,

    /// Coach 카드 보관함
    pub coach_cards: HashMap<String, Vec<CoachCard>>,

    /// Tactics 카드 보관함
    pub tactics_cards: HashMap<String, Vec<TacticsCard>>,

    /// 카드 컬렉션 (도감용)
    pub collection: HashMap<CardType, Vec<String>>,

    /// 보유 카드 총 개수
    pub total_counts: CardCounts,

    /// 인벤토리 용량
    pub max_capacity: usize,
}

/// 카드 개수 통계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardCounts {
    pub managers: usize,
    pub coaches: usize,
    pub tactics: usize,
    pub total: usize,
}

impl UnifiedInventory {
    /// 새 인벤토리 생성
    pub fn new() -> Self {
        Self {
            manager_cards: HashMap::new(),
            coach_cards: HashMap::new(),
            tactics_cards: HashMap::new(),
            collection: HashMap::new(),
            total_counts: CardCounts {
                managers: 0,
                coaches: 0,
                tactics: 0,
                total: 0,
            },
            max_capacity: 1000,
        }
    }

    /// GachaCard 추가 (가챠 결과 처리)
    pub fn add_gacha_card(&mut self, card: GachaCard) -> Result<bool, String> {
        if self.total_counts.total >= self.max_capacity {
            return Err("인벤토리가 가득 찼습니다.".to_string());
        }

        let is_new = match card {
            GachaCard::Coach(coach_card) => {
                self.add_coach_card(coach_card)
            },
            GachaCard::Tactics(tactics_card) => {
                self.add_tactics_card(tactics_card)
            }
        };

        Ok(is_new)
    }

    /// Coach 카드 추가 (Manager 포함)
    fn add_coach_card(&mut self, card: CoachCard) -> bool {
        let is_new = !self.has_card(&card.id);

        // 컬렉션 업데이트
        if is_new {
            self.collection
                .entry(card.card_type)
                .or_insert_with(Vec::new)
                .push(card.id.clone());
        }

        // 타입별로 분류하여 저장
        match card.card_type {
            CardType::Manager => {
                self.manager_cards
                    .entry(card.id.clone())
                    .or_insert_with(Vec::new)
                    .push(card);
                self.total_counts.managers += 1;
            },
            CardType::Coach => {
                self.coach_cards
                    .entry(card.id.clone())
                    .or_insert_with(Vec::new)
                    .push(card);
                self.total_counts.coaches += 1;
            },
            _ => {} // Tactics는 별도 처리
        }

        self.total_counts.total += 1;
        is_new
    }

    /// Tactics 카드 추가
    fn add_tactics_card(&mut self, card: TacticsCard) -> bool {
        let is_new = !self.has_card(&card.id);

        // 컬렉션 업데이트
        if is_new {
            self.collection
                .entry(CardType::Tactics)
                .or_insert_with(Vec::new)
                .push(card.id.clone());
        }

        // 전술 카드 저장
        self.tactics_cards
            .entry(card.id.clone())
            .or_insert_with(Vec::new)
            .push(card);

        self.total_counts.tactics += 1;
        self.total_counts.total += 1;
        is_new
    }

    /// 카드 보유 확인
    pub fn has_card(&self, card_id: &str) -> bool {
        self.manager_cards.contains_key(card_id) ||
        self.coach_cards.contains_key(card_id) ||
        self.tactics_cards.contains_key(card_id)
    }

    /// 덱에 필요한 카드 가져오기
    pub fn get_cards_for_deck(&self, deck: &Deck) -> DeckCards {
        DeckCards {
            manager: deck.manager_card.as_ref().and_then(|card| {
                self.manager_cards.get(&card.id)
                    .and_then(|cards| cards.first().cloned())
            }),
            coaches: deck.coach_cards.iter().map(|slot| {
                slot.as_ref().and_then(|card| {
                    self.coach_cards.get(&card.id)
                        .and_then(|cards| cards.first().cloned())
                })
            }).collect(),
            tactics: deck.tactics_cards.iter().map(|slot| {
                slot.as_ref().and_then(|card| {
                    self.tactics_cards.get(&card.id)
                        .and_then(|cards| cards.first().cloned())
                })
            }).collect(),
        }
    }

    /// Manager 카드 목록 가져오기
    pub fn get_manager_cards(&self) -> Vec<CoachCard> {
        self.manager_cards.values()
            .flat_map(|cards| cards.iter().cloned())
            .collect()
    }

    /// Coach 카드 목록 가져오기
    pub fn get_coach_cards(&self) -> Vec<CoachCard> {
        self.coach_cards.values()
            .flat_map(|cards| cards.iter().cloned())
            .collect()
    }

    /// Tactics 카드 목록 가져오기
    pub fn get_tactics_cards(&self) -> Vec<TacticsCard> {
        self.tactics_cards.values()
            .flat_map(|cards| cards.iter().cloned())
            .collect()
    }

    /// 레어도별 카드 가져오기
    pub fn get_cards_by_rarity(&self, rarity: CardRarity) -> CardsByType {
        CardsByType {
            managers: self.get_manager_cards()
                .into_iter()
                .filter(|c| c.rarity == rarity)
                .collect(),
            coaches: self.get_coach_cards()
                .into_iter()
                .filter(|c| c.rarity == rarity)
                .collect(),
            tactics: self.get_tactics_cards()
                .into_iter()
                .filter(|c| c.rarity == rarity)
                .collect(),
        }
    }

    /// 카드 제거 (덱에서 사용 중인 경우 제거 불가)
    pub fn remove_card(&mut self, card_id: &str, card_type: CardType) -> Result<(), String> {
        match card_type {
            CardType::Manager => {
                if let Some(cards) = self.manager_cards.get_mut(card_id) {
                    if cards.len() > 1 {
                        cards.pop();
                        self.total_counts.managers -= 1;
                        self.total_counts.total -= 1;
                        Ok(())
                    } else {
                        Err("덱에서 사용 중인 카드는 제거할 수 없습니다.".to_string())
                    }
                } else {
                    Err("카드를 찾을 수 없습니다.".to_string())
                }
            },
            CardType::Coach => {
                if let Some(cards) = self.coach_cards.get_mut(card_id) {
                    if cards.len() > 1 {
                        cards.pop();
                        self.total_counts.coaches -= 1;
                        self.total_counts.total -= 1;
                        Ok(())
                    } else {
                        Err("덱에서 사용 중인 카드는 제거할 수 없습니다.".to_string())
                    }
                } else {
                    Err("카드를 찾을 수 없습니다.".to_string())
                }
            },
            CardType::Tactics => {
                if let Some(cards) = self.tactics_cards.get_mut(card_id) {
                    if cards.len() > 1 {
                        cards.pop();
                        self.total_counts.tactics -= 1;
                        self.total_counts.total -= 1;
                        Ok(())
                    } else {
                        Err("덱에서 사용 중인 카드는 제거할 수 없습니다.".to_string())
                    }
                } else {
                    Err("카드를 찾을 수 없습니다.".to_string())
                }
            }
        }
    }

    /// 인벤토리 요약 정보
    pub fn get_summary(&self) -> String {
        format!(
            "카드 보유 현황:\n감독: {}장\n코치: {}장\n전술: {}장\n총 {}장 / {}",
            self.total_counts.managers,
            self.total_counts.coaches,
            self.total_counts.tactics,
            self.total_counts.total,
            self.max_capacity
        )
    }
}

/// 덱에 사용할 카드들
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCards {
    pub manager: Option<CoachCard>,
    pub coaches: Vec<Option<CoachCard>>,
    pub tactics: Vec<Option<TacticsCard>>,
}

/// 타입별 카드 목록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardsByType {
    pub managers: Vec<CoachCard>,
    pub coaches: Vec<CoachCard>,
    pub tactics: Vec<TacticsCard>,
}

/// 인벤토리 필터 옵션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOptions {
    pub card_type: Option<CardType>,
    pub rarity: Option<CardRarity>,
    pub specialty: Option<Specialty>,
    pub tactical_style: Option<TacticalStyle>,
}

impl Default for UnifiedInventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_inventory() {
        let mut inv = UnifiedInventory::new();

        // Manager 카드 추가
        let manager = CoachCard::new(
            "mgr_001".to_string(),
            "Test Manager".to_string(),
            CardRarity::Three,
            CardType::Manager,
            Specialty::Balanced,
            "Test".to_string(),
        );

        let is_new = inv.add_coach_card(manager.clone());
        assert!(is_new);
        assert_eq!(inv.total_counts.managers, 1);

        // Tactics 카드 추가
        let tactics = TacticsCard::new(
            "tac_001".to_string(),
            "Test Tactics".to_string(),
            CardRarity::Four,
            TacticalStyle::Attacking,
            "Test".to_string(),
        );

        let is_new = inv.add_tactics_card(tactics);
        assert!(is_new);
        assert_eq!(inv.total_counts.tactics, 1);

        // 총 개수 확인
        assert_eq!(inv.total_counts.total, 2);
    }

    #[test]
    fn test_gacha_card_handling() {
        let mut inv = UnifiedInventory::new();

        // Coach 타입 GachaCard
        let coach_card = GachaCard::Coach(CoachCard::new(
            "coach_001".to_string(),
            "Test Coach".to_string(),
            CardRarity::Two,
            CardType::Coach,
            Specialty::Speed,
            "Test".to_string(),
        ));

        let result = inv.add_gacha_card(coach_card);
        assert!(result.is_ok());
        assert_eq!(inv.total_counts.coaches, 1);

        // Tactics 타입 GachaCard
        let tactics_card = GachaCard::Tactics(TacticsCard::new(
            "tactics_001".to_string(),
            "Test Tactics".to_string(),
            CardRarity::Three,
            TacticalStyle::Possession,
            "Test".to_string(),
        ));

        let result = inv.add_gacha_card(tactics_card);
        assert!(result.is_ok());
        assert_eq!(inv.total_counts.tactics, 1);
    }
}
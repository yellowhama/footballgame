// 카드 인벤토리 시스템
use super::card::{CardRarity, CardType, CoachCard, Specialty};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 카드 인벤토리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInventory {
    /// 보유 카드 목록 (카드 ID -> 카드 리스트)
    pub owned_cards: HashMap<String, Vec<CoachCard>>,
    /// 전체 카드 수
    pub total_card_count: usize,
    /// 최대 보관 가능 수
    pub max_capacity: usize,
    /// 수집한 고유 카드 ID 목록
    pub collection: std::collections::HashSet<String>,
}

impl CardInventory {
    /// 새 인벤토리 생성
    pub fn new() -> Self {
        Self {
            owned_cards: HashMap::new(),
            total_card_count: 0,
            max_capacity: 500, // 최대 500장
            collection: std::collections::HashSet::new(),
        }
    }

    /// 카드 추가
    pub fn add_card(&mut self, card: CoachCard) -> Result<bool, String> {
        // 용량 체크
        if self.total_card_count >= self.max_capacity {
            return Err("인벤토리가 가득 찼습니다.".to_string());
        }

        // 새로운 카드인지 체크
        let is_new = !self.collection.contains(&card.id);
        if is_new {
            self.collection.insert(card.id.clone());
        }

        // 카드 추가
        self.owned_cards.entry(card.id.clone()).or_default().push(card);

        self.total_card_count += 1;

        Ok(is_new)
    }

    /// 여러 카드 추가
    pub fn add_cards(&mut self, cards: Vec<CoachCard>) -> Result<Vec<bool>, String> {
        let mut results = Vec::new();

        for card in cards {
            results.push(self.add_card(card)?);
        }

        Ok(results)
    }

    /// 카드 제거
    pub fn remove_card(&mut self, card_id: &str, index: usize) -> Result<CoachCard, String> {
        let cards = self
            .owned_cards
            .get_mut(card_id)
            .ok_or_else(|| "카드를 찾을 수 없습니다.".to_string())?;

        if index >= cards.len() {
            return Err("유효하지 않은 인덱스입니다.".to_string());
        }

        let card = cards.remove(index);
        self.total_card_count -= 1;

        // 해당 ID 카드가 모두 없어지면 엔트리 제거
        if cards.is_empty() {
            self.owned_cards.remove(card_id);
        }

        Ok(card)
    }

    /// 특정 카드 보유 수량
    pub fn get_card_count(&self, card_id: &str) -> usize {
        self.owned_cards.get(card_id).map(|cards| cards.len()).unwrap_or(0)
    }

    /// 카드 합성 (동일 카드 3장 -> 레벨업)
    pub fn merge_cards(&mut self, card_id: &str) -> Result<CoachCard, String> {
        let cards = self
            .owned_cards
            .get_mut(card_id)
            .ok_or_else(|| "카드를 찾을 수 없습니다.".to_string())?;

        if cards.len() < 3 {
            return Err("합성하려면 동일 카드 3장이 필요합니다.".to_string());
        }

        // 가장 높은 레벨 카드를 기준으로
        let base_index =
            cards.iter().enumerate().max_by_key(|(_, c)| c.level).map(|(i, _)| i).unwrap();

        let mut base_card = cards[base_index].clone();

        // 다른 카드들의 경험치 합산
        let mut total_exp = base_card.experience;
        for (i, card) in cards.iter().enumerate() {
            if i != base_index {
                total_exp += card.experience + 100 * card.level as u32;
            }
        }

        // 카드 3장 제거
        for _ in 0..3 {
            cards.pop();
        }
        self.total_card_count -= 3;

        // 합성된 카드 레벨업
        base_card.level = (base_card.level + 1).min(10);
        base_card.experience = total_exp;
        base_card.check_level_up();

        // 합성된 카드 추가
        cards.push(base_card.clone());
        self.total_card_count += 1;

        Ok(base_card)
    }

    /// 수집률 계산
    pub fn collection_rate(&self, total_cards: usize) -> f32 {
        self.collection.len() as f32 / total_cards as f32 * 100.0
    }

    /// 레어도별 카드 수
    pub fn count_by_rarity(&self) -> HashMap<CardRarity, usize> {
        let mut counts = HashMap::new();

        for cards in self.owned_cards.values() {
            for card in cards {
                *counts.entry(card.rarity).or_insert(0) += 1;
            }
        }

        counts
    }

    /// 타입별 카드 수
    pub fn count_by_type(&self) -> (usize, usize) {
        let mut manager_count = 0;
        let mut coach_count = 0;

        for cards in self.owned_cards.values() {
            for card in cards {
                match card.card_type {
                    CardType::Manager => manager_count += 1,
                    CardType::Coach => coach_count += 1,
                    CardType::Tactics => {} // Tactics cards not counted here
                }
            }
        }

        (manager_count, coach_count)
    }

    /// 전문분야별 베스트 카드 찾기
    pub fn get_best_by_specialty(&self, specialty: Specialty) -> Option<CoachCard> {
        self.owned_cards
            .values()
            .flatten()
            .filter(|c| c.specialty == specialty)
            .max_by_key(|c| (c.rarity as u8, c.level))
            .cloned()
    }

    /// 중복 카드 목록
    pub fn get_duplicates(&self) -> Vec<(String, Vec<CoachCard>)> {
        self.owned_cards
            .iter()
            .filter(|(_, cards)| cards.len() > 1)
            .map(|(id, cards)| (id.clone(), cards.clone()))
            .collect()
    }

    /// 정렬된 카드 목록
    pub fn get_sorted_cards(&self, sort_by: SortOption) -> Vec<CoachCard> {
        let mut all_cards: Vec<CoachCard> = self.owned_cards.values().flatten().cloned().collect();

        match sort_by {
            SortOption::Rarity => {
                all_cards.sort_by_key(|c| std::cmp::Reverse(c.rarity as u8));
            }
            SortOption::Level => {
                all_cards.sort_by_key(|c| std::cmp::Reverse(c.level));
            }
            SortOption::Name => {
                all_cards.sort_by(|a, b| a.name.cmp(&b.name));
            }
            SortOption::Specialty => {
                all_cards.sort_by_key(|c| c.specialty as u8);
            }
        }

        all_cards
    }

    /// 인벤토리 정리 (중복 카드 자동 합성)
    pub fn auto_merge_duplicates(&mut self) -> Vec<CoachCard> {
        let mut merged_cards = Vec::new();
        let duplicate_ids: Vec<String> = self
            .owned_cards
            .iter()
            .filter(|(_, cards)| cards.len() >= 3)
            .map(|(id, _)| id.clone())
            .collect();

        for card_id in duplicate_ids {
            // 레벨 10 카드는 제외
            let cards = self.owned_cards.get(&card_id).unwrap();
            if cards.iter().any(|c| c.level >= 10) {
                continue;
            }

            // 3장씩 합성
            while self.get_card_count(&card_id) >= 3 {
                if let Ok(merged) = self.merge_cards(&card_id) {
                    merged_cards.push(merged);
                } else {
                    break;
                }
            }
        }

        merged_cards
    }

    /// 인벤토리 요약
    pub fn summary(&self) -> String {
        let (managers, coaches) = self.count_by_type();
        let rarity_counts = self.count_by_rarity();

        format!(
            "📦 카드 인벤토리\n\
            총 카드: {}/{}\n\
            수집률: {:.1}%\n\
            감독: {}장, 코치: {}장\n\
            ⭐5: {}장, ⭐4: {}장, ⭐3: {}장",
            self.total_card_count,
            self.max_capacity,
            self.collection_rate(80), // 전체 80종 기준
            managers,
            coaches,
            rarity_counts.get(&CardRarity::Five).unwrap_or(&0),
            rarity_counts.get(&CardRarity::Four).unwrap_or(&0),
            rarity_counts.get(&CardRarity::Three).unwrap_or(&0),
        )
    }
}

impl Default for CardInventory {
    fn default() -> Self {
        Self::new()
    }
}

/// 정렬 옵션
#[derive(Debug, Clone, Copy)]
pub enum SortOption {
    Rarity,
    Level,
    Name,
    Specialty,
}

// 중복 정의 제거 - card.rs에 이미 있음

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_add_remove() {
        let mut inventory = CardInventory::new();

        let card = CoachCard::new(
            "test_001".to_string(),
            "테스트 카드".to_string(),
            CardRarity::Three,
            CardType::Coach,
            Specialty::Speed,
            "테스트".to_string(),
        );

        // 추가
        let is_new = inventory.add_card(card.clone()).unwrap();
        assert!(is_new);
        assert_eq!(inventory.total_card_count, 1);

        // 중복 추가
        let is_new = inventory.add_card(card).unwrap();
        assert!(!is_new); // 이미 있는 카드
        assert_eq!(inventory.total_card_count, 2);

        // 제거
        inventory.remove_card("test_001", 0).unwrap();
        assert_eq!(inventory.total_card_count, 1);
    }

    #[test]
    fn test_card_merge() {
        let mut inventory = CardInventory::new();

        // 같은 카드 3장 추가
        for _ in 0..3 {
            let card = CoachCard::new(
                "merge_test".to_string(),
                "합성 테스트".to_string(),
                CardRarity::Two,
                CardType::Coach,
                Specialty::Power,
                "테스트".to_string(),
            );
            inventory.add_card(card).unwrap();
        }

        assert_eq!(inventory.get_card_count("merge_test"), 3);

        // 합성
        let merged = inventory.merge_cards("merge_test").unwrap();
        // 합성: 레벨 1→2 + 경험치 200 (2장 × 100) → check_level_up으로 레벨 3
        assert_eq!(merged.level, 3);
        assert_eq!(inventory.get_card_count("merge_test"), 1); // 1장만 남음
    }

    #[test]
    fn test_collection_rate() {
        let mut inventory = CardInventory::new();

        // 서로 다른 카드 10종 추가
        for i in 1..=10 {
            let card = CoachCard::new(
                format!("card_{}", i),
                format!("카드 {}", i),
                CardRarity::One,
                CardType::Coach,
                Specialty::Balanced,
                "테스트".to_string(),
            );
            inventory.add_card(card).unwrap();
        }

        // 80종 중 10종 수집 = 12.5%
        assert_eq!(inventory.collection_rate(80), 12.5);
    }
}

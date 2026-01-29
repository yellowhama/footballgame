// 덱 편성 시스템
use super::card::{create_default_coach, create_default_manager, CardType, CoachCard, Specialty};
use super::tactics::{TacticalStyle, TacticsCard};
use crate::training::{CoachBonusLog, TrainingTarget};
use serde::{Deserialize, Serialize};

/// 덱 구성 (감독 1 + 코치 3 + 전술 3 = 7칸)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    /// 덱 이름
    pub name: String,
    /// 감독 카드 (1장)
    pub manager_card: Option<CoachCard>,
    /// 코치 카드 (3장)
    pub coach_cards: Vec<Option<CoachCard>>,
    /// 전술 카드 (3장)
    pub tactics_cards: Vec<Option<TacticsCard>>,
    /// 마지막 사용 시간
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl Deck {
    /// 새 덱 생성
    pub fn new(name: String) -> Self {
        Self {
            name,
            manager_card: None,
            coach_cards: vec![None, None, None],   // 3개 슬롯
            tactics_cards: vec![None, None, None], // 3개 슬롯
            last_used: None,
        }
    }

    /// 감독 카드 설정
    pub fn set_manager(&mut self, card: CoachCard) -> Result<(), String> {
        if card.card_type != CardType::Manager {
            return Err("감독 카드만 설정 가능합니다.".to_string());
        }
        self.manager_card = Some(card);
        Ok(())
    }

    /// 코치 카드 설정 (슬롯 0-2)
    pub fn set_coach(&mut self, slot: usize, card: CoachCard) -> Result<(), String> {
        if slot >= 3 {
            return Err("코치 슬롯은 0-2까지입니다.".to_string());
        }
        if card.card_type != CardType::Coach {
            return Err("코치 카드만 설정 가능합니다.".to_string());
        }

        // 중복 체크
        if self.has_duplicate(&card) {
            return Err("동일한 카드는 중복 편성할 수 없습니다.".to_string());
        }

        self.coach_cards[slot] = Some(card);
        Ok(())
    }

    /// 전술 카드 설정 (슬롯 0-2)
    pub fn set_tactics(&mut self, slot: usize, card: TacticsCard) -> Result<(), String> {
        if slot >= 3 {
            return Err("전술 슬롯은 0-2까지입니다.".to_string());
        }

        // 중복 체크 (전술 카드 ID)
        if self.has_duplicate_tactics(&card) {
            return Err("동일한 전술 카드는 중복 편성할 수 없습니다.".to_string());
        }

        self.tactics_cards[slot] = Some(card);
        Ok(())
    }

    /// 중복 카드 체크
    fn has_duplicate(&self, card: &CoachCard) -> bool {
        // 감독과 중복 체크
        if let Some(ref manager) = self.manager_card {
            if manager.id == card.id {
                return true;
            }
        }

        // 다른 코치와 중복 체크
        for coach in self.coach_cards.iter().flatten() {
            if coach.id == card.id {
                return true;
            }
        }

        false
    }

    /// 전술 카드 중복 체크
    fn has_duplicate_tactics(&self, card: &TacticsCard) -> bool {
        for tactics in self.tactics_cards.iter().flatten() {
            if tactics.id == card.id {
                return true;
            }
        }
        false
    }

    /// 훈련 보너스 계산
    pub fn calculate_training_bonus(&self, training_type: &TrainingTarget) -> f32 {
        self.calculate_training_bonus_with_log(training_type).0
    }

    pub fn calculate_training_bonus_with_log(
        &self,
        training_type: &TrainingTarget,
    ) -> (f32, Vec<CoachBonusLog>) {
        let mut total_bonus = 1.0;
        let mut logs = Vec::new();

        // 감독 보너스
        let default_manager = create_default_manager();
        let manager = self.manager_card.as_ref().unwrap_or(&default_manager);
        let manager_bonus = manager.current_bonus();
        total_bonus *= manager_bonus;
        logs.push(CoachBonusLog {
            source: format!("Manager: {}", manager.name),
            bonus_multiplier: manager_bonus,
            reason: "Manager card bonus".to_string(),
        });

        // 코치 보너스
        for (idx, slot) in self.coach_cards.iter().enumerate() {
            let coach = match slot {
                Some(c) => c.clone(),
                None => create_default_coach(Specialty::Balanced),
            };

            let specialty_multiplier =
                if coach.specialty.matches_training(training_type) { 2.0 } else { 1.0 };
            let combined = coach.current_bonus() * specialty_multiplier;
            total_bonus *= combined;

            let source = match slot {
                Some(c) => format!("Coach Slot {}: {}", idx + 1, c.name),
                None => format!("Coach Slot {} (default)", idx + 1),
            };
            let reason = match slot {
                Some(_) if specialty_multiplier > 1.0 => "Coach bonus + specialty match",
                Some(_) => "Coach card bonus",
                None => "Default coach placeholder",
            }
            .to_string();

            logs.push(CoachBonusLog { source, bonus_multiplier: combined, reason });
        }

        // 시너지 보너스
        let synergy = self.calculate_synergy_bonus();
        total_bonus *= synergy;
        if (synergy - 1.0).abs() > f32::EPSILON {
            logs.push(CoachBonusLog {
                source: "Synergy Bonus".to_string(),
                bonus_multiplier: synergy,
                reason: "Specialty/tactics/full deck synergy".to_string(),
            });
        }

        (total_bonus, logs)
    }

    /// 시너지 보너스 계산
    pub fn calculate_synergy_bonus(&self) -> f32 {
        let mut synergy = 1.0;

        // 같은 전문 분야 카드 수 계산
        let mut specialty_count = std::collections::HashMap::new();

        if let Some(ref manager) = self.manager_card {
            *specialty_count.entry(manager.specialty).or_insert(0) += 1;
        }

        for coach in self.coach_cards.iter().flatten() {
            *specialty_count.entry(coach.specialty).or_insert(0) += 1;
        }

        // 3장 이상 같은 전문분야: 10% 보너스
        for (_, count) in specialty_count {
            if count >= 3 {
                synergy *= 1.10;
            }
        }

        // 전술 콤보 체크
        let active_tactics = self.get_active_tactics();
        synergy *= self.calculate_tactical_combo_bonus(&active_tactics);

        // 모든 슬롯이 채워진 경우: 10% 보너스 (7칸 기준)
        if self.is_complete() {
            synergy *= 1.10;
        }

        // 모든 카드가 레어도 3 이상: 10% 보너스
        if self.all_high_rarity() {
            synergy *= 1.10;
        }

        synergy
    }

    /// 활성화된 전술 스타일 목록
    fn get_active_tactics(&self) -> Vec<TacticalStyle> {
        self.tactics_cards
            .iter()
            .filter_map(|slot| slot.as_ref().map(|t| t.tactical_style))
            .collect()
    }

    /// 전술 콤보 보너스 계산
    fn calculate_tactical_combo_bonus(&self, active_tactics: &[TacticalStyle]) -> f32 {
        let combos = super::tactics::get_predefined_combos();
        let mut bonus = 1.0;

        for combo in combos {
            if combo.is_active(active_tactics) {
                bonus *= 1.0 + combo.bonus_value;
            }
        }

        bonus
    }

    /// 덱 완성 여부 (7칸 모두 채워짐)
    pub fn is_complete(&self) -> bool {
        self.manager_card.is_some()
            && self.coach_cards.iter().all(|c| c.is_some())
            && self.tactics_cards.iter().all(|t| t.is_some())
    }

    /// 모든 카드가 고레어도(3+)인지
    fn all_high_rarity(&self) -> bool {
        let manager = match self.manager_card.as_ref() {
            Some(m) => m,
            None => return false,
        };
        if (manager.rarity as u8) < 3 {
            return false;
        }

        if !self.coach_cards.iter().all(|c| c.is_some()) {
            return false;
        }

        for coach in self.coach_cards.iter().flatten() {
            if (coach.rarity as u8) < 3 {
                return false;
            }
        }

        true
    }

    /// 덱 사용 기록
    pub fn record_use(&mut self) {
        self.last_used = Some(chrono::Utc::now());

        // 모든 카드 사용 횟수 증가
        if let Some(ref mut manager) = self.manager_card {
            manager.record_use();
        }

        for ref mut coach in self.coach_cards.iter_mut().flatten() {
            coach.record_use();
        }

        for ref mut tactics in self.tactics_cards.iter_mut().flatten() {
            tactics.use_count += 1;
        }
    }

    /// 덱 요약 정보
    pub fn summary(&self) -> String {
        let manager_name = self
            .manager_card
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "빈 슬롯".to_string());

        let coach_names: Vec<String> = self
            .coach_cards
            .iter()
            .map(|slot| {
                slot.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "빈 슬롯".to_string())
            })
            .collect();

        let tactics_names: Vec<String> = self
            .tactics_cards
            .iter()
            .map(|slot| {
                slot.as_ref().map(|t| t.name.clone()).unwrap_or_else(|| "빈 슬롯".to_string())
            })
            .collect();

        format!(
            "덱: {}\n감독: {}\n코치: {}, {}, {}\n전술: {}, {}, {}",
            self.name,
            manager_name,
            coach_names[0],
            coach_names[1],
            coach_names[2],
            tactics_names[0],
            tactics_names[1],
            tactics_names[2]
        )
    }

    /// 덱 검증
    pub fn validate(&self) -> Result<(), String> {
        // 최소 감독은 있어야 함
        if self.manager_card.is_none() {
            return Err("감독 카드가 필요합니다.".to_string());
        }

        // 중복 체크
        let mut card_ids = std::collections::HashSet::new();

        if let Some(ref manager) = self.manager_card {
            card_ids.insert(&manager.id);
        }

        for coach in self.coach_cards.iter().flatten() {
            if !card_ids.insert(&coach.id) {
                return Err("중복된 카드가 있습니다.".to_string());
            }
        }

        Ok(())
    }
}

/// 덱 컬렉션 관리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCollection {
    /// 저장된 덱들 (최대 5개)
    pub decks: Vec<Deck>,
    /// 현재 활성 덱 인덱스
    pub active_deck: Option<usize>,
}

impl DeckCollection {
    pub fn new() -> Self {
        Self { decks: Vec::new(), active_deck: None }
    }

    /// 덱 추가 (최대 5개)
    pub fn add_deck(&mut self, deck: Deck) -> Result<(), String> {
        if self.decks.len() >= 5 {
            return Err("최대 5개의 덱만 저장 가능합니다.".to_string());
        }

        // 이름 중복 체크
        if self.decks.iter().any(|d| d.name == deck.name) {
            return Err("동일한 이름의 덱이 이미 존재합니다.".to_string());
        }

        self.decks.push(deck);

        // 첫 번째 덱이면 자동으로 활성화
        if self.decks.len() == 1 {
            self.active_deck = Some(0);
        }

        Ok(())
    }

    /// 활성 덱 설정
    pub fn set_active_deck(&mut self, index: usize) -> Result<(), String> {
        if index >= self.decks.len() {
            return Err("유효하지 않은 덱 인덱스입니다.".to_string());
        }
        self.active_deck = Some(index);
        Ok(())
    }

    /// 활성 덱 가져오기
    pub fn get_active_deck(&self) -> Option<&Deck> {
        self.active_deck.and_then(|idx| self.decks.get(idx))
    }

    /// 활성 덱 가져오기 (가변)
    pub fn get_active_deck_mut(&mut self) -> Option<&mut Deck> {
        self.active_deck.and_then(move |idx| self.decks.get_mut(idx))
    }
}

impl Default for DeckCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::card::CardRarity;
    use super::*;
    use crate::training::TrainingTarget;

    #[test]
    fn test_deck_creation() {
        let deck = Deck::new("테스트 덱".to_string());
        assert_eq!(deck.coach_cards.len(), 3);
        assert!(deck.manager_card.is_none());
    }

    #[test]
    fn test_deck_bonus_calculation() {
        let mut deck = Deck::new("테스트 덱".to_string());

        // 감독 설정
        let manager = CoachCard::new(
            "m001".to_string(),
            "명장".to_string(),
            CardRarity::Three,
            CardType::Manager,
            Specialty::Balanced,
            "모든 훈련 효과 증가".to_string(),
        );
        deck.set_manager(manager).unwrap();

        // 스피드 전문 코치 추가
        let coach = CoachCard::new(
            "c001".to_string(),
            "스피드 코치".to_string(),
            CardRarity::Two,
            CardType::Coach,
            Specialty::Speed,
            "속도 훈련 전문".to_string(),
        );
        deck.set_coach(0, coach).unwrap();

        // 스피드 훈련 보너스 계산
        let bonus = deck.calculate_training_bonus(&TrainingTarget::Pace);
        assert!(bonus > 1.0); // 보너스가 적용되어야 함
    }

    #[test]
    fn test_deck_collection() {
        let mut collection = DeckCollection::new();

        let deck1 = Deck::new("덱1".to_string());
        let deck2 = Deck::new("덱2".to_string());

        collection.add_deck(deck1).unwrap();
        collection.add_deck(deck2).unwrap();

        assert_eq!(collection.decks.len(), 2);
        assert_eq!(collection.active_deck, Some(0));

        collection.set_active_deck(1).unwrap();
        assert_eq!(collection.active_deck, Some(1));
    }
}

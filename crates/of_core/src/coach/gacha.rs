// 가챠 시스템
use super::card::{CardRarity, CardType, CoachCard, Specialty};
use super::tactics::{TacticalStyle, TacticsCard};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// 통합 카드 타입 (가챠 결과용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GachaCard {
    Coach(CoachCard),
    Tactics(TacticsCard),
}

impl GachaCard {
    /// 카드 ID 반환
    pub fn id(&self) -> &str {
        match self {
            GachaCard::Coach(c) => &c.id,
            GachaCard::Tactics(t) => &t.id,
        }
    }

    /// 카드 타입 반환
    pub fn card_type(&self) -> CardType {
        match self {
            GachaCard::Coach(c) => c.card_type,
            GachaCard::Tactics(_) => CardType::Tactics,
        }
    }

    /// 레어도 반환
    pub fn rarity(&self) -> CardRarity {
        match self {
            GachaCard::Coach(c) => c.rarity,
            GachaCard::Tactics(t) => t.rarity,
        }
    }

    /// 카드 이름 반환
    pub fn name(&self) -> &str {
        match self {
            GachaCard::Coach(c) => &c.name,
            GachaCard::Tactics(t) => &t.name,
        }
    }

    /// 디스플레이용 문자열
    pub fn display(&self) -> String {
        match self {
            GachaCard::Coach(c) => c.display(),
            GachaCard::Tactics(t) => {
                format!("{} {} {}", t.rarity.emoji(), t.name, t.tactical_style.icon())
            }
        }
    }
}

/// 가챠 풀 (뽑기 가능한 카드 목록)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GachaPool {
    /// 일반 풀 (상시)
    pub regular_cards: Vec<GachaCard>,
    /// 픽업 풀 (기간한정/이벤트)
    pub pickup_cards: Vec<GachaCard>,
    /// 픽업 확률 증가율
    pub pickup_rate_boost: f32,
}

impl Default for GachaPool {
    fn default() -> Self {
        Self::new()
    }
}

impl GachaPool {
    /// 새 가챠 풀 생성
    pub fn new() -> Self {
        Self {
            regular_cards: Self::create_default_pool(),
            pickup_cards: Vec::new(),
            pickup_rate_boost: 2.0, // 픽업 시 2배 확률
        }
    }

    /// 기본 카드 풀 생성
    fn create_default_pool() -> Vec<GachaCard> {
        let mut cards = Vec::new();

        // 감독 카드 20종
        for i in 1..=20 {
            let rarity = match i {
                1..=10 => CardRarity::One,
                11..=15 => CardRarity::Two,
                16..=18 => CardRarity::Three,
                19 => CardRarity::Four,
                20 => CardRarity::Five,
                _ => CardRarity::One,
            };

            let specialty = match i % 5 {
                0 => Specialty::Speed,
                1 => Specialty::Power,
                2 => Specialty::Technical,
                3 => Specialty::Mental,
                _ => Specialty::Balanced,
            };

            cards.push(GachaCard::Coach(CoachCard::new(
                format!("manager_{:03}", i),
                format!("감독 {}", i),
                rarity,
                CardType::Manager,
                specialty,
                format!("{:?} 훈련 전문 감독", specialty),
            )));
        }

        // 코치 카드 60종
        for i in 1..=60 {
            let rarity = match i {
                1..=30 => CardRarity::One,
                31..=45 => CardRarity::Two,
                46..=54 => CardRarity::Three,
                55..=58 => CardRarity::Four,
                59..=60 => CardRarity::Five,
                _ => CardRarity::One,
            };

            let specialty = match i % 5 {
                0 => Specialty::Speed,
                1 => Specialty::Power,
                2 => Specialty::Technical,
                3 => Specialty::Mental,
                _ => Specialty::Balanced,
            };

            cards.push(GachaCard::Coach(CoachCard::new(
                format!("coach_{:03}", i),
                format!("코치 {}", i),
                rarity,
                CardType::Coach,
                specialty,
                format!("{:?} 훈련 전문 코치", specialty),
            )));
        }

        // 전술 카드 40종 추가
        for i in 1..=40 {
            let rarity = match i {
                1..=20 => CardRarity::One,
                21..=30 => CardRarity::Two,
                31..=36 => CardRarity::Three,
                37..=39 => CardRarity::Four,
                40 => CardRarity::Five,
                _ => CardRarity::One,
            };

            let tactical_style = match i % 8 {
                0 => TacticalStyle::Defensive,
                1 => TacticalStyle::Balanced,
                2 => TacticalStyle::Attacking,
                3 => TacticalStyle::CounterAttack,
                4 => TacticalStyle::Possession,
                5 => TacticalStyle::Pressing,
                6 => TacticalStyle::DirectPlay,
                _ => TacticalStyle::WingPlay,
            };

            cards.push(GachaCard::Tactics(TacticsCard::new(
                format!("tactics_{:03}", i),
                format!("전술 {}", i),
                rarity,
                tactical_style,
                format!("{} 전술", tactical_style.description()),
            )));
        }

        cards
    }

    /// 픽업 카드 설정
    pub fn set_pickup(&mut self, cards: Vec<GachaCard>) {
        self.pickup_cards = cards;
    }
}

/// 가챠 시스템
pub struct GachaSystem {
    pub pool: GachaPool,
    pub pity_counter: u32,   // 천장 카운터
    pub pity_threshold: u32, // 천장 임계값 (보통 100)
}

impl GachaSystem {
    pub fn new() -> Self {
        Self { pool: GachaPool::new(), pity_counter: 0, pity_threshold: 100 }
    }

    /// 단일 뽑기
    pub fn pull_single(&mut self, seed: u64) -> GachaResult {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

        self.pity_counter += 1;

        // 천장 체크
        if self.pity_counter >= self.pity_threshold {
            self.pity_counter = 0;
            return self.pull_guaranteed_high_rarity(&mut rng);
        }

        // 레어도 결정
        let rarity = self.determine_rarity(&mut rng);

        // 카드 선택
        let card = self.select_card_by_rarity(rarity, &mut rng);

        // ⭐4 이상이면 천장 리셋
        if rarity as u8 >= 4 {
            self.pity_counter = 0;
        }

        GachaResult {
            cards: vec![card],
            is_new_flags: vec![true], // 기본값, check_new_cards()로 실제 체크
        }
    }

    /// 10연차
    pub fn pull_ten(&mut self, seed: u64) -> GachaResult {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let mut cards = Vec::new();

        // 9번 일반 뽑기
        for i in 0..9 {
            let subseed = seed.wrapping_add(i);
            let result = self.pull_single(subseed);
            cards.extend(result.cards);
        }

        // 10번째는 ⭐3 이상 보장
        self.pity_counter += 1;
        let guaranteed = self.pull_guaranteed_three_star_plus(&mut rng);
        cards.push(guaranteed);

        let is_new_flags = vec![true; cards.len()]; // 기본값, check_new_cards()로 실제 체크
        GachaResult { cards, is_new_flags }
    }

    /// 레어도 결정
    fn determine_rarity(&self, rng: &mut impl Rng) -> CardRarity {
        let roll = rng.gen::<f32>();

        // 기본 확률
        match roll {
            r if r < 0.03 => CardRarity::Five,  // 3%
            r if r < 0.10 => CardRarity::Four,  // 7%
            r if r < 0.25 => CardRarity::Three, // 15%
            r if r < 0.50 => CardRarity::Two,   // 25%
            _ => CardRarity::One,               // 50%
        }
    }

    /// 특정 레어도에서 카드 선택
    fn select_card_by_rarity(&self, rarity: CardRarity, rng: &mut impl Rng) -> GachaCard {
        // 픽업 체크
        if !self.pool.pickup_cards.is_empty() && rng.gen::<f32>() < 0.5 {
            // 50% 확률로 픽업 풀에서 선택
            let pickup_candidates: Vec<_> =
                self.pool.pickup_cards.iter().filter(|c| c.rarity() == rarity).collect();

            if !pickup_candidates.is_empty() {
                let idx = rng.gen_range(0..pickup_candidates.len());
                return pickup_candidates[idx].clone();
            }
        }

        // 일반 풀에서 선택
        let candidates: Vec<_> =
            self.pool.regular_cards.iter().filter(|c| c.rarity() == rarity).collect();

        if !candidates.is_empty() {
            let idx = rng.gen_range(0..candidates.len());
            candidates[idx].clone()
        } else {
            // 해당 레어도 카드가 없으면 기본 카드
            GachaCard::Coach(CoachCard::new(
                format!("default_{:?}", rarity),
                "기본 카드".to_string(),
                rarity,
                CardType::Coach,
                Specialty::Balanced,
                "기본 카드입니다.".to_string(),
            ))
        }
    }

    /// ⭐3 이상 보장 뽑기
    fn pull_guaranteed_three_star_plus(&self, rng: &mut impl Rng) -> GachaCard {
        let roll = rng.gen::<f32>();

        let rarity = match roll {
            r if r < 0.10 => CardRarity::Five, // 10%
            r if r < 0.30 => CardRarity::Four, // 20%
            _ => CardRarity::Three,            // 70%
        };

        self.select_card_by_rarity(rarity, rng)
    }

    /// 천장 보장 (⭐4 이상)
    fn pull_guaranteed_high_rarity(&self, rng: &mut impl Rng) -> GachaResult {
        let roll = rng.gen::<f32>();

        let rarity = if roll < 0.3 {
            CardRarity::Five // 30%
        } else {
            CardRarity::Four // 70%
        };

        let card = self.select_card_by_rarity(rarity, rng);

        GachaResult {
            cards: vec![card],
            is_new_flags: vec![true], // 기본값, check_new_cards()로 실제 체크
        }
    }
}

/// 가챠 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GachaResult {
    pub cards: Vec<GachaCard>,
    /// 각 카드별 신규 여부 (인벤토리 체크 후 설정)
    pub is_new_flags: Vec<bool>,
}

impl GachaResult {
    /// 인벤토리 컬렉션을 기반으로 신규 카드 여부 체크
    pub fn check_new_cards(&mut self, collection: &std::collections::HashSet<String>) {
        self.is_new_flags = self.cards.iter().map(|card| !collection.contains(card.id())).collect();
    }

    /// 신규 카드가 하나라도 있는지 확인
    pub fn has_new_cards(&self) -> bool {
        self.is_new_flags.iter().any(|&is_new| is_new)
    }

    /// 신규 카드 수
    pub fn new_card_count(&self) -> usize {
        self.is_new_flags.iter().filter(|&&is_new| is_new).count()
    }

    /// 결과 요약
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("뽑기 결과: {}장\n", self.cards.len()));

        // 레어도별 카운트
        let mut rarity_count = [0; 5];
        for card in &self.cards {
            rarity_count[card.rarity() as usize - 1] += 1;
        }

        for (i, count) in rarity_count.iter().enumerate() {
            if *count > 0 {
                let rarity = match i {
                    0 => "⭐",
                    1 => "⭐⭐",
                    2 => "⭐⭐⭐",
                    3 => "⭐⭐⭐⭐",
                    _ => "⭐⭐⭐⭐⭐",
                };
                summary.push_str(&format!("{}: {}장\n", rarity, count));
            }
        }

        // 최고 레어 카드 표시
        if let Some(best) = self.cards.iter().max_by_key(|c| c.rarity() as u8) {
            summary.push_str(&format!("\n최고 레어: {}", best.display()));
        }

        // 신규 카드 수 표시
        let new_count = self.new_card_count();
        if new_count > 0 {
            summary.push_str(&format!("\n✨ 신규 카드: {}장", new_count));
        }

        summary
    }

    /// 애니메이션용 연출 순서 (레어도 낮은 것부터)
    pub fn animation_order(&self) -> Vec<GachaCard> {
        let mut cards = self.cards.clone();
        cards.sort_by_key(|c| c.rarity() as u8);
        cards
    }
}

impl Default for GachaSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_pull() {
        let mut gacha = GachaSystem::new();
        let result = gacha.pull_single(42);

        assert_eq!(result.cards.len(), 1);
    }

    #[test]
    fn test_ten_pull() {
        let mut gacha = GachaSystem::new();
        let result = gacha.pull_ten(42);

        assert_eq!(result.cards.len(), 10);

        // 최소 1장은 ⭐3 이상이어야 함
        let high_rarity_count = result.cards.iter().filter(|c| c.rarity() as u8 >= 3).count();
        assert!(high_rarity_count >= 1);
    }

    #[test]
    fn test_pity_system() {
        let mut gacha = GachaSystem::new();
        gacha.pity_counter = 99; // 천장 직전

        let result = gacha.pull_single(42);

        // 천장 도달로 ⭐4 이상 보장
        assert!(result.cards[0].rarity() as u8 >= 4);
        assert_eq!(gacha.pity_counter, 0); // 리셋됨
    }

    #[test]
    fn test_check_new_cards() {
        let mut gacha = GachaSystem::new();
        let mut result = gacha.pull_ten(42);

        // 초기 상태: 모든 카드가 신규
        assert_eq!(result.is_new_flags.len(), 10);
        assert!(result.is_new_flags.iter().all(|&x| x));
        assert_eq!(result.new_card_count(), 10);

        // 컬렉션에 일부 카드 추가
        let mut collection = std::collections::HashSet::new();
        collection.insert(result.cards[0].id().to_string());
        collection.insert(result.cards[1].id().to_string());

        // 인벤토리 체크
        result.check_new_cards(&collection);

        // 첫 2장은 이미 보유, 나머지는 신규
        assert!(!result.is_new_flags[0]);
        assert!(!result.is_new_flags[1]);
        assert_eq!(result.new_card_count(), 8);
        assert!(result.has_new_cards());
    }
}

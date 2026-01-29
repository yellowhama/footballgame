// 체력 포인트 관리 시스템 (0-100)
use serde::{Deserialize, Serialize};

/// 체력 상태 관리
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaminaSystem {
    /// 현재 체력 (0-100)
    current: u8,
    /// 최대 체력 (기본 100, 특성에 따라 변동 가능)
    maximum: u8,
    /// 일일 회복량 (기본 30)
    recovery_rate: u8,
}

impl StaminaSystem {
    /// 새로운 체력 시스템 생성
    pub fn new() -> Self {
        Self { current: 100, maximum: 100, recovery_rate: 30 }
    }

    /// 커스텀 설정으로 생성
    pub fn with_config(maximum: u8, recovery_rate: u8) -> Self {
        Self { current: maximum, maximum, recovery_rate }
    }

    /// 현재 체력 반환
    pub fn current(&self) -> u8 {
        self.current
    }

    /// 체력 퍼센티지 반환
    pub fn percentage(&self) -> f32 {
        (self.current as f32 / self.maximum as f32) * 100.0
    }

    /// 체력 소모
    pub fn consume(&mut self, amount: u8) -> Result<(), StaminaError> {
        if amount > self.current {
            return Err(StaminaError::InsufficientStamina {
                required: amount,
                available: self.current,
            });
        }
        self.current = self.current.saturating_sub(amount);
        Ok(())
    }

    /// 체력 회복 (휴식)
    pub fn rest(&mut self) {
        self.current = (self.current + self.recovery_rate).min(self.maximum);
    }

    /// 부분 회복
    pub fn recover(&mut self, amount: u8) {
        self.current = (self.current + amount).min(self.maximum);
    }

    /// 체력 완전 회복
    pub fn full_recover(&mut self) {
        self.current = self.maximum;
    }

    /// 체력 상태 체크
    pub fn status(&self) -> StaminaStatus {
        match self.current {
            80..=100 => StaminaStatus::Excellent,
            60..=79 => StaminaStatus::Good,
            40..=59 => StaminaStatus::Normal,
            20..=39 => StaminaStatus::Tired,
            _ => StaminaStatus::Exhausted,
        }
    }

    /// 부상 위험도 계산
    pub fn injury_risk(&self) -> f32 {
        match self.current {
            40..=100 => 0.01, // 1% 기본 위험
            30..=39 => 0.05,  // 5%
            20..=29 => 0.15,  // 15%
            10..=19 => 0.30,  // 30%
            5..=9 => 0.50,    // 50%
            _ => 0.80,        // 80% 극도 위험
        }
    }

    /// 훈련 가능 여부 체크
    pub fn can_train(&self, required_stamina: u8) -> bool {
        self.current >= required_stamina && self.current >= 10 // 최소 10 필요
    }

    /// 권장 훈련 강도 제안
    pub fn recommended_intensity(&self) -> TrainingIntensity {
        match self.current {
            70..=100 => TrainingIntensity::Intensive,
            40..=69 => TrainingIntensity::Normal,
            20..=39 => TrainingIntensity::Light,
            _ => TrainingIntensity::Rest,
        }
    }
}

/// 체력 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaminaStatus {
    Excellent,
    Good,
    Normal,
    Tired,
    Exhausted,
}

impl StaminaStatus {
    pub fn display_text(&self) -> &'static str {
        match self {
            StaminaStatus::Excellent => "최상 💪",
            StaminaStatus::Good => "좋음 👍",
            StaminaStatus::Normal => "보통 😊",
            StaminaStatus::Tired => "피곤 😓",
            StaminaStatus::Exhausted => "탈진 😵",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            StaminaStatus::Excellent => "green",
            StaminaStatus::Good => "blue",
            StaminaStatus::Normal => "white",
            StaminaStatus::Tired => "yellow",
            StaminaStatus::Exhausted => "red",
        }
    }
}

/// 훈련 강도
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingIntensity {
    Rest,      // 휴식 권장
    Light,     // 가벼운 훈련 (10-20 소모)
    Normal,    // 보통 훈련 (20-30 소모)
    Intensive, // 집중 훈련 (30-40 소모)
}

impl TrainingIntensity {
    pub fn stamina_cost(&self) -> u8 {
        match self {
            TrainingIntensity::Rest => 0,
            TrainingIntensity::Light => 15,
            TrainingIntensity::Normal => 25,
            TrainingIntensity::Intensive => 40,
        }
    }

    pub fn effect_multiplier(&self) -> f32 {
        match self {
            TrainingIntensity::Rest => 0.0,
            TrainingIntensity::Light => 0.6,
            TrainingIntensity::Normal => 1.0,
            TrainingIntensity::Intensive => 1.5,
        }
    }
}

/// 체력 관련 에러
#[derive(Debug, Clone, PartialEq)]
pub enum StaminaError {
    InsufficientStamina { required: u8, available: u8 },
}

impl std::fmt::Display for StaminaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaminaError::InsufficientStamina { required, available } => {
                write!(f, "체력 부족: 필요 {}, 현재 {}", required, available)
            }
        }
    }
}

impl std::error::Error for StaminaError {}

impl Default for StaminaSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stamina_consumption() {
        let mut stamina = StaminaSystem::new();
        assert_eq!(stamina.current(), 100);

        // 정상 소모
        stamina.consume(30).unwrap();
        assert_eq!(stamina.current(), 70);

        // 과도한 소모 시도
        let result = stamina.consume(80);
        assert!(result.is_err());
        assert_eq!(stamina.current(), 70); // 변화 없음
    }

    #[test]
    fn test_recovery() {
        let mut stamina = StaminaSystem::new();
        stamina.consume(60).unwrap();
        assert_eq!(stamina.current(), 40);

        stamina.rest();
        assert_eq!(stamina.current(), 70); // 30 회복

        stamina.rest();
        assert_eq!(stamina.current(), 100); // 최대치 제한
    }

    #[test]
    fn test_injury_risk() {
        let mut stamina = StaminaSystem::new();
        assert_eq!(stamina.injury_risk(), 0.01); // 체력 100

        stamina.consume(70).unwrap();
        assert_eq!(stamina.injury_risk(), 0.05); // 체력 30

        stamina.consume(25).unwrap();
        assert_eq!(stamina.injury_risk(), 0.50); // 체력 5 (5..=9 범위)
    }
}

//! Special Skill System
//!
//! Handles skill score calculation, unlock conditions, and effects

use crate::engine::physics_constants::skills;
use crate::models::{Player, SpecialSkill};

/// 스킬 점수 계산 및 습득 조건 체크
pub trait SkillCalculator {
    /// 특정 스킬의 점수 계산 (0.0~1.0)
    fn get_skill_score(&self, skill: SpecialSkill) -> f32;

    /// 스킬 습득 가능 여부 (점수 0.80 이상 = 16/20 이상)
    fn can_unlock_skill(&self, skill: SpecialSkill) -> bool;

    /// 스킬 보유 여부
    fn has_skill(&self, skill: SpecialSkill) -> bool;

    /// 자동으로 모든 습득 가능한 스킬을 장착
    fn auto_equip_skills(&mut self);
}

impl SkillCalculator for Player {
    fn get_skill_score(&self, skill: SpecialSkill) -> f32 {
        // attributes가 None이면 0.0 반환
        let Some(ref attr) = self.attributes else {
            return 0.0;
        };

        match skill {
            // === 멘탈/창의성 기반 스킬 ===
            SpecialSkill::AnkleBreaker => {
                let flair = skills::normalize(attr.flair as f32);
                let technique = skills::normalize(attr.technique as f32);
                let dribbling = skills::normalize(attr.dribbling as f32);
                (flair * 0.4) + (technique * 0.3) + (dribbling * 0.3)
            }

            SpecialSkill::Maestro => {
                let flair = skills::normalize(attr.flair as f32);
                let vision = skills::normalize(attr.vision as f32);
                let passing = skills::normalize(attr.passing as f32);
                (flair * 0.4) + (vision * 0.3) + (passing * 0.3)
            }

            SpecialSkill::Panenka => {
                let flair = skills::normalize(attr.flair as f32);
                let composure = skills::normalize(attr.composure as f32);
                let technique = skills::normalize(attr.technique as f32);
                (flair * 0.4) + (composure * 0.35) + (technique * 0.25)
            }

            SpecialSkill::CurveArtist => {
                let flair = skills::normalize(attr.flair as f32);
                let crossing = skills::normalize(attr.crossing as f32);
                let technique = skills::normalize(attr.technique as f32);
                (flair * 0.4) + (crossing * 0.35) + (technique * 0.25)
            }

            // === 물리/기술 기반 스킬 ===
            SpecialSkill::FinesseShot => {
                let long_shots = skills::normalize(attr.long_shots as f32);
                let technique = skills::normalize(attr.technique as f32);
                let free_kicks = skills::normalize(attr.free_kicks as f32);
                (long_shots * 0.4) + (technique * 0.3) + (free_kicks * 0.3)
            }

            SpecialSkill::PowerHeader => {
                let heading = skills::normalize(attr.heading as f32);
                let strength = skills::normalize(attr.strength as f32);
                let jumping = skills::normalize(attr.jumping as f32); // Note: 'jumping' not 'jumping_reach'
                (heading * 0.4) + (strength * 0.3) + (jumping * 0.3)
            }

            SpecialSkill::Poacher => {
                let off_the_ball = skills::normalize(attr.off_the_ball as f32);
                let acceleration = skills::normalize(attr.acceleration as f32);
                let anticipation = skills::normalize(attr.anticipation as f32);
                (off_the_ball * 0.4) + (acceleration * 0.3) + (anticipation * 0.3)
            }

            SpecialSkill::PerfectTackle => {
                let tackling = skills::normalize(attr.tackling as f32);
                let decisions = skills::normalize(attr.decisions as f32);
                let agility = skills::normalize(attr.agility as f32);
                (tackling * 0.5) + (decisions * 0.3) + (agility * 0.2)
            }

            SpecialSkill::Cannon => {
                let strength = skills::normalize(attr.strength as f32);
                let long_shots = skills::normalize(attr.long_shots as f32);
                let finishing = skills::normalize(attr.finishing as f32);
                (strength * 0.4) + (long_shots * 0.35) + (finishing * 0.25)
            }

            SpecialSkill::SpeedDemon => {
                let pace = skills::normalize(attr.pace as f32);
                let acceleration = skills::normalize(attr.acceleration as f32);
                let dribbling = skills::normalize(attr.dribbling as f32);
                (pace * 0.45) + (acceleration * 0.35) + (dribbling * 0.2)
            }
        }
    }

    fn can_unlock_skill(&self, skill: SpecialSkill) -> bool {
        self.get_skill_score(skill) >= 0.80
    }

    fn has_skill(&self, skill: SpecialSkill) -> bool {
        self.equipped_skills.contains(&skill)
    }

    fn auto_equip_skills(&mut self) {
        self.equipped_skills.clear();

        // 모든 스킬을 순회하며 습득 가능한 것 자동 장착
        let all_skills = [
            SpecialSkill::AnkleBreaker,
            SpecialSkill::Maestro,
            SpecialSkill::Panenka,
            SpecialSkill::CurveArtist,
            SpecialSkill::FinesseShot,
            SpecialSkill::PowerHeader,
            SpecialSkill::Poacher,
            SpecialSkill::PerfectTackle,
            SpecialSkill::Cannon,
            SpecialSkill::SpeedDemon,
        ];

        for skill in all_skills {
            if self.can_unlock_skill(skill) {
                self.equipped_skills.push(skill);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;
    use crate::models::player::Position;
    use crate::models::trait_system::TraitSlots;

    #[test]
    fn test_skill_score_calculation() {
        let mut player = Player {
            name: "Test Player".to_string(),
            position: Position::ST,
            overall: 85,
            condition: 3,
            attributes: Some(PlayerAttributes::default()),
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: Default::default(),
        };

        // 앵클 브레이커 조건 설정 (Flair 18, Technique 16, Dribbling 16)
        if let Some(ref mut attr) = player.attributes {
            attr.flair = 18;
            attr.technique = 16;
            attr.dribbling = 16;
        }

        let score = player.get_skill_score(SpecialSkill::AnkleBreaker);

        // 예상: (18/20 * 0.4) + (16/20 * 0.3) + (16/20 * 0.3) = 0.36 + 0.24 + 0.24 = 0.84
        assert!(score >= 0.80, "Score should be above unlock threshold: {}", score);
        assert!(player.can_unlock_skill(SpecialSkill::AnkleBreaker));
    }

    #[test]
    fn test_auto_equip_skills() {
        let mut player = Player {
            name: "Test Player".to_string(),
            position: Position::CAM,
            overall: 90,
            condition: 3,
            attributes: Some(PlayerAttributes::default()),
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: Default::default(),
        };

        // 여러 스킬 습득 조건 설정
        if let Some(ref mut attr) = player.attributes {
            attr.flair = 18;
            attr.technique = 18;
            attr.dribbling = 18;
            attr.vision = 18;
            attr.passing = 18;
        }

        player.auto_equip_skills();

        assert!(!player.equipped_skills.is_empty(), "Should have equipped at least one skill");
        assert!(player.has_skill(SpecialSkill::AnkleBreaker));
        assert!(player.has_skill(SpecialSkill::Maestro));
    }

    #[test]
    fn test_no_attributes_returns_zero() {
        let player = Player {
            name: "No Attrs".to_string(),
            position: Position::ST,
            overall: 50,
            condition: 3,
            attributes: None,
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: Default::default(),
        };

        let score = player.get_skill_score(SpecialSkill::AnkleBreaker);
        assert_eq!(score, 0.0, "Should return 0.0 when attributes are None");
    }
}

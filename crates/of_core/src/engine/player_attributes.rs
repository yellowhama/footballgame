//! Player Attribute Accessor Methods
//!
//! match_sim.rs에서 추출된 선수 속성 접근 메서드들.
//! MatchEngine의 impl 블록을 분리하여 파일 크기 감소.
//!
//! ## 포함 함수 (~35개)
//! - get_player_* 속성 getter들 (long_shots, finishing, composure, etc.)
//! - get_player_attribute, get_player_attribute_with_trait (핵심 헬퍼)
//! - 트레이트 관련: get_trait_action_multiplier, player_has_trait, player_has_gold_trait
//! - 유틸: is_user_player, get_player, get_player_name

use super::match_sim::MatchEngine;
use crate::models::trait_system::{ActionType as TraitActionType, StatType, TraitId};
use crate::models::TeamSide;

impl MatchEngine {
    // ========================================================================
    // Player Attribute Getters (with Trait bonuses)
    // ========================================================================

    pub(crate) fn get_player_long_shots(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.long_shots as f32,
            StatType::LongShots,
            10.0,
        )
    }

    pub(crate) fn get_player_finishing(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.finishing as f32,
            StatType::Finishing,
            10.0,
        )
    }

    pub(crate) fn get_player_composure(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.composure as f32,
            StatType::Composure,
            10.0,
        )
    }

    pub(crate) fn get_player_technique(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.technique as f32, 10.0)
    }

    pub(crate) fn get_player_strength(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.strength as f32,
            StatType::Strength,
            10.0,
        )
    }

    pub(crate) fn get_player_passing(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.passing as f32,
            StatType::Passing,
            10.0,
        )
    }

    pub(crate) fn get_player_vision(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.vision as f32,
            StatType::Vision,
            10.0,
        )
    }

    pub(crate) fn get_player_anticipation(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.anticipation as f32,
            StatType::Anticipation,
            10.0,
        )
    }

    pub(crate) fn get_player_concentration(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.concentration as f32, 10.0)
    }

    pub(crate) fn get_player_off_the_ball(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.off_the_ball as f32, 10.0)
    }

    pub(crate) fn get_player_positioning(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.positioning as f32,
            StatType::Positioning,
            10.0,
        )
    }

    pub(crate) fn get_player_teamwork(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.teamwork as f32, 10.0)
    }

    pub(crate) fn get_player_acceleration(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.acceleration as f32,
            StatType::Acceleration,
            10.0,
        )
    }

    // Technical attributes
    pub(crate) fn get_player_corners(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.corners as f32,
            StatType::Corners,
            10.0,
        )
    }

    pub(crate) fn get_player_crossing(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.crossing as f32,
            StatType::Crossing,
            10.0,
        )
    }

    pub(crate) fn get_player_dribbling(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.dribbling as f32,
            StatType::Dribbling,
            10.0,
        )
    }

    pub(crate) fn get_player_first_touch(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.first_touch as f32,
            StatType::FirstTouch,
            10.0,
        )
    }

    pub(crate) fn get_player_heading(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.heading as f32,
            StatType::Heading,
            10.0,
        )
    }

    pub(crate) fn get_player_long_throws(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.long_throws as f32, 10.0)
    }

    pub(crate) fn get_player_marking(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.marking as f32,
            StatType::Marking,
            10.0,
        )
    }

    pub(crate) fn get_player_penalty_taking(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.penalty_taking as f32,
            StatType::Penalties,
            10.0,
        )
    }

    pub(crate) fn get_player_tackling(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.tackling as f32,
            StatType::Tackling,
            10.0,
        )
    }

    // Mental attributes
    pub(crate) fn get_player_aggression(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.aggression as f32,
            StatType::Aggression,
            10.0,
        )
    }

    pub(crate) fn get_player_bravery(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.bravery as f32, 10.0)
    }

    pub(crate) fn get_player_decisions(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.decisions as f32, 10.0)
    }

    pub(crate) fn get_player_determination(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.determination as f32, 10.0)
    }

    pub(crate) fn get_player_leadership(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(player_idx, |attrs| attrs.leadership as f32, 10.0)
    }

    pub(crate) fn get_player_work_rate(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.work_rate as f32,
            StatType::WorkRate,
            10.0,
        )
    }

    // Physical attributes
    pub(crate) fn get_player_pace(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.pace as f32,
            StatType::Pace,
            10.0,
        )
    }

    pub(crate) fn get_player_stamina(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.stamina as f32,
            StatType::Stamina,
            10.0,
        )
    }

    pub(crate) fn get_player_agility(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.agility as f32,
            StatType::Agility,
            10.0,
        )
    }

    pub(crate) fn get_player_jumping(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.jumping as f32,
            StatType::Jumping,
            10.0,
        )
    }

    pub(crate) fn get_player_balance(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.balance as f32,
            StatType::Balance,
            10.0,
        )
    }

    pub(crate) fn get_player_flair(&self, player_idx: usize) -> f32 {
        self.get_player_attribute_with_trait(
            player_idx,
            |attrs| attrs.flair as f32,
            StatType::Flair,
            10.0,
        )
    }

    // ========================================================================
    // Goalkeeper-Specific Attributes (v5 schema)
    // ========================================================================

    /// GK reflexes - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_reflexes(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_reflexes > 0 {
                    attrs.gk_reflexes as f32
                } else {
                    // Fallback: derive from anticipation + agility + concentration
                    let sum = attrs.anticipation as u16 + attrs.agility as u16 + attrs.concentration as u16;
                    (sum / 3) as f32
                }
            },
            10.0,
        )
    }

    /// GK handling - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_handling(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_handling > 0 {
                    attrs.gk_handling as f32
                } else {
                    // Fallback: derive from first_touch + composure + concentration
                    let sum = attrs.first_touch as u16 + attrs.composure as u16 + attrs.concentration as u16;
                    (sum / 3) as f32
                }
            },
            10.0,
        )
    }

    /// GK one-on-ones - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_one_on_ones(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_one_on_ones > 0 {
                    attrs.gk_one_on_ones as f32
                } else {
                    // Fallback: derive from anticipation + bravery + decisions
                    let sum = attrs.anticipation as u16 + attrs.bravery as u16 + attrs.decisions as u16;
                    (sum / 3) as f32
                }
            },
            10.0,
        )
    }

    /// GK aerial reach - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_aerial_reach(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_aerial_reach > 0 {
                    attrs.gk_aerial_reach as f32
                } else {
                    // Fallback: derive from jumping + strength
                    let sum = attrs.jumping as u16 + attrs.strength as u16;
                    (sum / 2) as f32
                }
            },
            10.0,
        )
    }

    /// GK command of area - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_command_of_area(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_command_of_area > 0 {
                    attrs.gk_command_of_area as f32
                } else {
                    // Fallback: derive from positioning + decisions + leadership
                    let sum = attrs.positioning as u16 + attrs.decisions as u16 + attrs.leadership as u16;
                    (sum / 3) as f32
                }
            },
            10.0,
        )
    }

    /// GK rushing out - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_rushing_out(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_rushing_out > 0 {
                    attrs.gk_rushing_out as f32
                } else {
                    // Fallback: derive from acceleration + bravery + decisions
                    let sum = attrs.acceleration as u16 + attrs.bravery as u16 + attrs.decisions as u16;
                    (sum / 3) as f32
                }
            },
            10.0,
        )
    }

    /// GK kicking - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_kicking(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_kicking > 0 {
                    attrs.gk_kicking as f32
                } else {
                    // Fallback: derive from passing + technique
                    let sum = attrs.passing as u16 + attrs.technique as u16;
                    (sum / 2) as f32
                }
            },
            10.0,
        )
    }

    /// GK throwing - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_throwing(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_throwing > 0 {
                    attrs.gk_throwing as f32
                } else {
                    // Fallback: derive from passing + strength
                    let sum = attrs.passing as u16 + attrs.strength as u16;
                    (sum / 2) as f32
                }
            },
            10.0,
        )
    }

    /// GK punching - v5에서 실제 값 사용, 없으면 파생
    pub(crate) fn get_player_gk_punching(&self, player_idx: usize) -> f32 {
        self.get_player_attribute(
            player_idx,
            |attrs| {
                if attrs.gk_punching > 0 {
                    attrs.gk_punching as f32
                } else {
                    // Fallback: derive from strength + bravery
                    let sum = attrs.strength as u16 + attrs.bravery as u16;
                    (sum / 2) as f32
                }
            },
            10.0,
        )
    }

    // ========================================================================
    // User Player Helpers
    // ========================================================================

    /// 주인공 선수인지 확인하는 헬퍼 함수
    pub(crate) fn is_user_player(&self, player_idx: usize, is_home: bool) -> bool {
        let user_config = match &self.user_player {
            Some(config) => config,
            None => return false,
        };

        // 팀이 맞는지 확인
        if user_config.is_home_team != is_home {
            return false;
        }

        // 선수 이름이 맞는지 확인 (SSOT: MatchSetup assignment-aware)
        if player_idx >= 22 {
            return false;
        }

        // Safety: callers should pass a consistent (player_idx, is_home) pair.
        if TeamSide::is_home(player_idx) != is_home {
            return false;
        }

        self.get_match_player(player_idx).name == user_config.player_name
    }

    /// 선수 객체 직접 접근 헬퍼
    pub(crate) fn get_player(
        &self,
        player_idx: usize,
    ) -> Option<&crate::models::MatchPlayer> {
        if player_idx >= 22 {
            return None;
        }
        Some(self.get_match_player(player_idx))
    }

    /// 선수 이름 조회 헬퍼
    pub(crate) fn get_player_name(&self, player_idx: usize) -> String {
        self.get_player(player_idx)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("Player_{}", player_idx))
    }

    // ========================================================================
    // Core Attribute Access (Generic Helpers)
    // ========================================================================

    /// 범용 속성 접근 헬퍼
    pub(crate) fn get_player_attribute<F>(&self, player_idx: usize, getter: F, default: f32) -> f32
    where
        F: Fn(&crate::models::player::PlayerAttributes) -> f32,
    {
        if player_idx >= 22 {
            return default;
        }

        let attrs = self.get_player_attributes(player_idx);
        getter(attrs)
    }

    /// 속성 접근 + Trait 시너지 보너스 적용 + 피로도 페널티
    /// stat_type: 해당 속성에 맞는 StatType (trait 보너스 계산용)
    pub(crate) fn get_player_attribute_with_trait<F>(
        &self,
        player_idx: usize,
        getter: F,
        stat_type: StatType,
        default: f32,
    ) -> f32
    where
        F: Fn(&crate::models::player::PlayerAttributes) -> f32,
    {
        let base = self.get_player_attribute(player_idx, getter, default);

        // Get trait bonus from player (SSOT: MatchSetup assignment-aware)
        if player_idx >= 22 {
            return base;
        }

        let traits = self.get_player_traits(player_idx);
        let trait_bonus = traits.get_stat_bonus(stat_type);        

        // ============================================
        // 피로도 페널티 (70분 이후부터)
        // ============================================
        let fatigue_penalty = if self.minute >= 70 {
            // Gold Engine: 70분 이후 피로 무효화
            if traits.has_gold_trait(TraitId::Engine) {
                0.0
            } else {
                // 70분 이후 분당 0.5% 페널티 (90분이면 10% 감소)
                // 스태미나가 높을수록 페널티 감소
                let stamina = self.get_player_attributes(player_idx).stamina as f32;
                let stamina_factor = 1.0 - (stamina / 20.0).min(0.5); // 스태미나 높으면 피로 적음
                let minutes_fatigued = (self.minute - 70) as f32;
                let base_penalty = minutes_fatigued * 0.005 * stamina_factor; // 분당 0.5% × 스태미나보정
                base_penalty.min(0.15) // 최대 15% 페널티
            }
        } else {
            0.0
        };

        let final_value = (base + trait_bonus) * (1.0 - fatigue_penalty);
        final_value.max(1.0) // 최소값 보장
    }

    // ========================================================================
    // Trait Helpers
    // ========================================================================

    /// Trait 활성화 효과 배율 가져오기
    pub(crate) fn get_trait_action_multiplier(
        &self,
        player_idx: usize,
        action: TraitActionType,
    ) -> f32 {
        if player_idx >= 22 {
            return 1.0;
        }

        self.get_player_traits(player_idx)
            .get_action_multiplier(action)
    }

    /// 플레이어가 특정 트레이트를 가지고 있는지 확인
    pub(crate) fn player_has_trait(&self, player_idx: usize, trait_id: TraitId) -> bool {
        if player_idx >= 22 {
            return false;
        }

        self.get_player_traits(player_idx).has_trait(trait_id)
    }

    /// Gold 티어 트레이트 체크 (스페셜 효과용)
    pub(crate) fn player_has_gold_trait(&self, player_idx: usize, trait_id: TraitId) -> bool {
        if player_idx >= 22 {
            return false;
        }

        self.get_player_traits(player_idx).has_gold_trait(trait_id)
    }
}

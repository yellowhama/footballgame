// 주간 훈련 계획 시스템
use crate::training::types::TrainingTarget;
use chrono::{NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

/// 요일
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    pub fn from_weekday(weekday: Weekday) -> Self {
        match weekday {
            Weekday::Mon => DayOfWeek::Monday,
            Weekday::Tue => DayOfWeek::Tuesday,
            Weekday::Wed => DayOfWeek::Wednesday,
            Weekday::Thu => DayOfWeek::Thursday,
            Weekday::Fri => DayOfWeek::Friday,
            Weekday::Sat => DayOfWeek::Saturday,
            Weekday::Sun => DayOfWeek::Sunday,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DayOfWeek::Monday => "월요일",
            DayOfWeek::Tuesday => "화요일",
            DayOfWeek::Wednesday => "수요일",
            DayOfWeek::Thursday => "목요일",
            DayOfWeek::Friday => "금요일",
            DayOfWeek::Saturday => "토요일",
            DayOfWeek::Sunday => "일요일",
        }
    }
}

/// 일일 활동 슬롯
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaySlot {
    /// 팀 훈련 (의무)
    TeamTraining(TrainingTarget),
    /// 자유 시간 (선택 가능)
    FreeTime,
    /// 경기
    Match { opponent: String, is_home: bool },
    /// 휴식 (강제/선택)
    Rest { forced: bool },
    /// 특별 이벤트
    SpecialEvent { name: String, description: String },
}

impl DaySlot {
    pub fn display_text(&self) -> String {
        match self {
            DaySlot::TeamTraining(target) => {
                format!("팀훈련: {}", target.display_name())
            }
            DaySlot::FreeTime => "자유시간".to_string(),
            DaySlot::Match { opponent, is_home } => {
                if *is_home {
                    format!("홈경기 vs {}", opponent)
                } else {
                    format!("원정경기 @ {}", opponent)
                }
            }
            DaySlot::Rest { forced } => {
                if *forced {
                    "강제휴식".to_string()
                } else {
                    "휴식".to_string()
                }
            }
            DaySlot::SpecialEvent { name, .. } => {
                format!("이벤트: {}", name)
            }
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DaySlot::TeamTraining(_) => "👥",
            DaySlot::FreeTime => "⭐",
            DaySlot::Match { .. } => "⚽",
            DaySlot::Rest { .. } => "😴",
            DaySlot::SpecialEvent { .. } => "🎉",
        }
    }
}

/// 주간 계획
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyPlan {
    /// 주차 번호 (1-156, 3년)
    pub week_number: u16,
    /// 시작 날짜
    pub start_date: NaiveDate,
    /// 각 요일별 계획
    pub schedule: Vec<(DayOfWeek, Vec<DaySlot>)>,
}

impl WeeklyPlan {
    /// 새 주간 계획 생성
    pub fn new(week_number: u16, start_date: NaiveDate) -> Self {
        // 기본 주간 계획 (월/수/금 팀훈련)
        let schedule = vec![
            (
                DayOfWeek::Monday,
                vec![DaySlot::TeamTraining(TrainingTarget::Balanced), DaySlot::FreeTime],
            ),
            (DayOfWeek::Tuesday, vec![DaySlot::FreeTime, DaySlot::FreeTime]),
            (
                DayOfWeek::Wednesday,
                vec![DaySlot::TeamTraining(TrainingTarget::Technical), DaySlot::FreeTime],
            ),
            (DayOfWeek::Thursday, vec![DaySlot::FreeTime, DaySlot::FreeTime]),
            (
                DayOfWeek::Friday,
                vec![DaySlot::TeamTraining(TrainingTarget::Endurance), DaySlot::FreeTime],
            ),
            (DayOfWeek::Saturday, vec![DaySlot::FreeTime, DaySlot::FreeTime]),
            (DayOfWeek::Sunday, vec![DaySlot::Rest { forced: false }]),
        ];

        Self { week_number, start_date, schedule }
    }

    /// 주차 번호만으로 기본 계획 생성 (시작 날짜는 기본값)
    pub fn default_for_week(week_number: u16) -> Self {
        // 기본 시작일 계산 (2024년 1월 1일 기준)
        let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let start_date = base_date + chrono::Duration::days((week_number as i64 - 1) * 7);
        Self::new(week_number, start_date)
    }

    /// 경기 일정 추가
    pub fn add_match(&mut self, day: DayOfWeek, opponent: String, is_home: bool) {
        // 해당 요일 찾기
        if let Some((_, slots)) = self.schedule.iter_mut().find(|(d, _)| *d == day) {
            // 경기 추가, 그날 다른 활동 제거
            slots.clear();
            slots.push(DaySlot::Match { opponent, is_home });

            // 경기 전날은 가벼운 훈련만
            self.adjust_pre_match_day(day);
        }
    }

    /// 경기 전날 조정
    fn adjust_pre_match_day(&mut self, match_day: DayOfWeek) {
        let pre_day = match self.get_previous_day(match_day) {
            Some(day) => day,
            None => return,
        };

        if let Some((_, slots)) = self.schedule.iter_mut().find(|(d, _)| *d == pre_day) {
            // 경기 전날은 가벼운 훈련이나 휴식만
            slots.clear();
            slots.push(DaySlot::TeamTraining(TrainingTarget::Technical));
            slots.push(DaySlot::Rest { forced: false });
        }
    }

    /// 전날 요일 구하기
    fn get_previous_day(&self, day: DayOfWeek) -> Option<DayOfWeek> {
        match day {
            DayOfWeek::Monday => None, // 주 시작
            DayOfWeek::Tuesday => Some(DayOfWeek::Monday),
            DayOfWeek::Wednesday => Some(DayOfWeek::Tuesday),
            DayOfWeek::Thursday => Some(DayOfWeek::Wednesday),
            DayOfWeek::Friday => Some(DayOfWeek::Thursday),
            DayOfWeek::Saturday => Some(DayOfWeek::Friday),
            DayOfWeek::Sunday => Some(DayOfWeek::Saturday),
        }
    }

    /// 팀 훈련 횟수 계산
    pub fn count_team_training(&self) -> usize {
        self.schedule
            .iter()
            .flat_map(|(_, slots)| slots)
            .filter(|slot| matches!(slot, DaySlot::TeamTraining(_)))
            .count()
    }

    /// 자유 시간 슬롯 수 계산
    pub fn count_free_slots(&self) -> usize {
        self.schedule
            .iter()
            .flat_map(|(_, slots)| slots)
            .filter(|slot| matches!(slot, DaySlot::FreeTime))
            .count()
    }

    /// 특정 요일의 활동 가져오기
    pub fn get_day_schedule(&self, day: DayOfWeek) -> Option<&Vec<DaySlot>> {
        self.schedule.iter().find(|(d, _)| *d == day).map(|(_, slots)| slots)
    }

    /// 주간 요약 텍스트
    pub fn summary_text(&self) -> String {
        format!(
            "{}주차 ({}~)\n팀훈련: {}회, 자유시간: {}슬롯",
            self.week_number,
            self.start_date.format("%m/%d"),
            self.count_team_training(),
            self.count_free_slots()
        )
    }
}

/// 주간 계획 생성기
pub struct WeeklyPlanGenerator {
    /// 시즌 시작 날짜
    season_start: NaiveDate,
    /// 경기 일정
    match_schedule: Vec<(u16, DayOfWeek, String, bool)>, // (week, day, opponent, is_home)
}

impl WeeklyPlanGenerator {
    pub fn new(season_start: NaiveDate) -> Self {
        Self { season_start, match_schedule: Vec::new() }
    }

    /// 경기 일정 추가
    pub fn add_match_schedule(
        &mut self,
        week: u16,
        day: DayOfWeek,
        opponent: String,
        is_home: bool,
    ) {
        self.match_schedule.push((week, day, opponent, is_home));
    }

    /// 특정 주차의 계획 생성
    pub fn generate_week(&self, week_number: u16) -> WeeklyPlan {
        let start_date = self.season_start + chrono::Duration::weeks((week_number - 1) as i64);
        let mut plan = WeeklyPlan::new(week_number, start_date);

        // 해당 주의 경기 일정 적용
        for (week, day, opponent, is_home) in &self.match_schedule {
            if *week == week_number {
                plan.add_match(*day, opponent.clone(), *is_home);
            }
        }

        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekly_plan_creation() {
        let date = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let plan = WeeklyPlan::new(1, date);

        assert_eq!(plan.week_number, 1);
        assert_eq!(plan.count_team_training(), 3); // 월/수/금
        assert!(plan.count_free_slots() > 0);
    }

    #[test]
    fn test_add_match() {
        let date = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let mut plan = WeeklyPlan::new(1, date);

        plan.add_match(DayOfWeek::Saturday, "라이벌고".to_string(), true);

        let saturday = plan.get_day_schedule(DayOfWeek::Saturday).unwrap();
        assert!(saturday.iter().any(|slot| matches!(slot, DaySlot::Match { .. })));
    }

    #[test]
    fn test_plan_generator() {
        let season_start = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let mut generator = WeeklyPlanGenerator::new(season_start);

        generator.add_match_schedule(2, DayOfWeek::Saturday, "강북고".to_string(), false);

        let week2 = generator.generate_week(2);
        assert_eq!(week2.week_number, 2);

        let saturday = week2.get_day_schedule(DayOfWeek::Saturday).unwrap();
        assert!(saturday.iter().any(|slot| {
            matches!(slot, DaySlot::Match { opponent, .. } if opponent == "강북고")
        }));
    }
}

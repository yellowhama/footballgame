//! Story System Localization
//!
//! Fluent (FTL) 기반 다국어 지원

use crate::error::CoreError;
use fluent::{FluentArgs, FluentBundle, FluentMessage, FluentResource, FluentValue};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use unic_langid::LanguageIdentifier;

/// 지원 언어
pub const SUPPORTED_LOCALES: &[&str] = &["en-US", "ko-KR", "ja-JP"];

/// 스토리 텍스트 로컬라이저
pub struct StoryLocalizer {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
    current_locale: String,
    fallback_locale: String,
}

impl Default for StoryLocalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryLocalizer {
    /// 새 로컬라이저 생성
    pub fn new() -> Self {
        Self {
            bundles: HashMap::new(),
            current_locale: "en-US".to_string(),
            fallback_locale: "en-US".to_string(),
        }
    }

    /// FTL 파일에서 리소스 로드
    pub fn load_from_dir(&mut self, dir_path: &Path) -> Result<(), CoreError> {
        for locale in SUPPORTED_LOCALES {
            let file_path = dir_path.join(format!("{}.ftl", locale));
            if file_path.exists() {
                let content = fs::read_to_string(&file_path)
                    .map_err(|e| CoreError::IoError(e.to_string()))?;

                self.load_locale(locale, &content)?;
            }
        }
        Ok(())
    }

    /// 특정 언어 리소스 로드
    pub fn load_locale(&mut self, locale: &str, ftl_content: &str) -> Result<(), CoreError> {
        let resource = FluentResource::try_new(ftl_content.to_string())
            .map_err(|_| CoreError::ParseError("Failed to parse FTL content".into()))?;

        let lang_id: LanguageIdentifier = locale
            .parse()
            .map_err(|_| CoreError::ParseError(format!("Invalid locale: {}", locale)))?;

        let mut bundle = FluentBundle::new(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|_| CoreError::ParseError("Failed to add resource to bundle".into()))?;

        self.bundles.insert(locale.to_string(), bundle);
        Ok(())
    }

    /// 현재 로케일 설정
    pub fn set_locale(&mut self, locale: &str) -> Result<(), CoreError> {
        if !self.bundles.contains_key(locale) {
            return Err(CoreError::NotFound(format!("Locale {} not loaded", locale)));
        }
        self.current_locale = locale.to_string();
        Ok(())
    }

    /// 자동 언어 협상
    pub fn negotiate_locale(&mut self, requested: &[&str]) -> String {
        let available: Vec<LanguageIdentifier> =
            self.bundles.keys().filter_map(|k| k.parse().ok()).collect();

        let requested: Vec<LanguageIdentifier> =
            requested.iter().filter_map(|l| l.parse().ok()).collect();

        let default: LanguageIdentifier = self.fallback_locale.parse().unwrap();

        let negotiated = negotiate_languages(
            &requested,
            &available,
            Some(&default),
            NegotiationStrategy::Filtering,
        );

        negotiated.first().map(|l| l.to_string()).unwrap_or_else(|| self.fallback_locale.clone())
    }

    /// 메시지 포맷팅
    pub fn format(&self, key: &str, args: Option<HashMap<String, FluentValue>>) -> String {
        if let Some(bundle) = self.bundles.get(&self.current_locale) {
            if let Some(message) = bundle.get_message(key) {
                return self.format_pattern(bundle, message, args);
            }
        }

        // Fallback 시도
        if self.current_locale != self.fallback_locale {
            if let Some(bundle) = self.bundles.get(&self.fallback_locale) {
                if let Some(message) = bundle.get_message(key) {
                    return self.format_pattern(bundle, message, args);
                }
            }
        }

        // 키 자체 반환
        format!("[{}]", key)
    }

    /// 패턴 포맷팅 헬퍼
    fn format_pattern(
        &self,
        bundle: &FluentBundle<FluentResource>,
        message: FluentMessage,
        args: Option<HashMap<String, FluentValue>>,
    ) -> String {
        let pattern = message.value().expect("Message has no value");
        let mut errors = vec![];

        let formatted = if let Some(hash_args) = args {
            let mut fluent_args = FluentArgs::new();
            for (key, value) in hash_args {
                fluent_args.set(key, value);
            }
            bundle.format_pattern(pattern, Some(&fluent_args), &mut errors)
        } else {
            bundle.format_pattern(pattern, None, &mut errors)
        };

        formatted.to_string()
    }

    /// 이벤트 텍스트 포맷팅
    pub fn format_event(&self, event_id: &str, context: &EventContext) -> LocalizedEvent {
        let title_key = format!("{}-title", event_id);
        let desc_key = format!("{}-desc", event_id);

        let mut args = HashMap::new();
        args.insert("player_name".to_string(), FluentValue::from(context.player_name.clone()));
        args.insert("team_name".to_string(), FluentValue::from(context.team_name.clone()));
        args.insert("week".to_string(), FluentValue::from(context.week as i64));

        LocalizedEvent {
            title: self.format(&title_key, Some(args.clone())),
            description: self.format(&desc_key, Some(args)),
            choices: context.choice_keys.iter().map(|key| self.format(key, None)).collect(),
        }
    }
}

/// 이벤트 컨텍스트
pub struct EventContext {
    pub player_name: String,
    pub team_name: String,
    pub week: u32,
    pub choice_keys: Vec<String>,
}

/// 로컬라이즈된 이벤트
pub struct LocalizedEvent {
    pub title: String,
    pub description: String,
    pub choices: Vec<String>,
}

/// 샘플 FTL 콘텐츠 생성
pub fn create_sample_ftl() -> HashMap<&'static str, &'static str> {
    let mut samples = HashMap::new();

    samples.insert(
        "en-US",
        r#"
# Story Events
training-breakthrough-title = Training Breakthrough
training-breakthrough-desc = { $player_name } has shown exceptional progress in training this week!

choice-train-harder = Train Even Harder
choice-rest = Take a Rest
choice-consult-coach = Consult with Coach

match-hero-title = Match Hero
match-hero-desc = { $player_name } was the hero of the match for { $team_name } in week { $week }!

career-milestone-title = Career Milestone
career-milestone-desc = { $player_name } has reached an important milestone in their career.
"#,
    );

    samples.insert(
        "ko-KR",
        r#"
# 스토리 이벤트
training-breakthrough-title = 훈련 돌파구
training-breakthrough-desc = { $player_name }가 이번 주 훈련에서 놀라운 발전을 보였습니다!

choice-train-harder = 더 열심히 훈련하기
choice-rest = 휴식 취하기
choice-consult-coach = 코치와 상담하기

match-hero-title = 경기 영웅
match-hero-desc = { $player_name }가 { $week }주차 { $team_name }의 경기 영웅이 되었습니다!

career-milestone-title = 커리어 이정표
career-milestone-desc = { $player_name }가 커리어의 중요한 이정표에 도달했습니다.
"#,
    );

    samples.insert(
        "ja-JP",
        r#"
# ストーリーイベント
training-breakthrough-title = トレーニングブレイクスルー
training-breakthrough-desc = { $player_name }は今週のトレーニングで素晴らしい進歩を見せました！

choice-train-harder = もっと頑張って練習する
choice-rest = 休憩を取る
choice-consult-coach = コーチと相談する

match-hero-title = 試合のヒーロー
match-hero-desc = { $player_name }は第{ $week }週の{ $team_name }の試合でヒーローになりました！

career-milestone-title = キャリアマイルストーン
career-milestone-desc = { $player_name }はキャリアの重要なマイルストーンに到達しました。
"#,
    );

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localizer_basic() {
        let mut localizer = StoryLocalizer::new();

        // Load English
        let en_content = create_sample_ftl()["en-US"];
        localizer.load_locale("en-US", en_content).unwrap();

        // Test formatting
        let mut args = HashMap::new();
        args.insert("player_name".to_string(), FluentValue::from("John Smith"));
        args.insert("team_name".to_string(), FluentValue::from("FC Example"));
        args.insert("week".to_string(), FluentValue::from(10));

        let formatted = localizer.format("match-hero-desc", Some(args));
        assert!(formatted.contains("John Smith"));
        assert!(formatted.contains("FC Example"));
    }

    #[test]
    fn test_locale_negotiation() {
        let mut localizer = StoryLocalizer::new();

        // Load multiple locales
        for (locale, content) in create_sample_ftl() {
            localizer.load_locale(locale, content).unwrap();
        }

        // Test negotiation
        let requested = vec!["ko", "en"];
        let negotiated = localizer.negotiate_locale(&requested);
        assert_eq!(negotiated, "ko-KR");
    }

    #[test]
    fn test_event_formatting() {
        let mut localizer = StoryLocalizer::new();
        localizer.load_locale("en-US", create_sample_ftl()["en-US"]).unwrap();

        let context = EventContext {
            player_name: "Test Player".to_string(),
            team_name: "Test Team".to_string(),
            week: 5,
            choice_keys: vec!["choice-train-harder".to_string(), "choice-rest".to_string()],
        };

        let event = localizer.format_event("training-breakthrough", &context);
        assert_eq!(event.title, "Training Breakthrough");
        assert!(event.description.contains("Test Player"));
        assert_eq!(event.choices.len(), 2);
    }
}

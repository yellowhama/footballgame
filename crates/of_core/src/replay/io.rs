use super::types::ReplayDoc;
use std::fs;
use std::path::Path;

/// 리플레이 JSON 저장
pub fn save_replay_json<P: AsRef<Path>>(doc: &ReplayDoc, path: P) -> anyhow::Result<()> {
    let data = serde_json::to_string_pretty(doc)?;
    fs::write(path, data)?;
    Ok(())
}

/// 리플레이 JSON 로드
pub fn load_replay_json<P: AsRef<Path>>(path: P) -> anyhow::Result<ReplayDoc> {
    let data = fs::read_to_string(path)?;
    let doc: ReplayDoc = serde_json::from_str(&data)?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{
        EventBase, PitchSpec, ReplayDoc, ReplayEvent, ReplayRosters, ReplayTeamsTactics,
    };
    use std::fs;

    #[test]
    fn test_save_load_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_replay.json");

        let doc = ReplayDoc {
            version: 1,
            pitch_m: PitchSpec { width_m: 105.0, height_m: 68.0 },
            events: vec![ReplayEvent::KickOff {
                base: EventBase { t: 0.0, player_id: None, team_id: Some(0) },
            }],
            rosters: ReplayRosters::default(),
            timeline: Vec::new(),
            tactics: ReplayTeamsTactics::default(),
        };

        // Save
        save_replay_json(&doc, &test_file).unwrap();
        assert!(test_file.exists());

        // Load
        let loaded_doc = load_replay_json(&test_file).unwrap();
        assert_eq!(doc, loaded_doc);

        // Cleanup
        fs::remove_file(&test_file).ok();
    }
}

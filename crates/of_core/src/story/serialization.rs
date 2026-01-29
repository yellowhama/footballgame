//! Story System Serialization
//!
//! MessagePack 기반 직렬화/역직렬화

use super::*;
use crate::error::CoreError;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// MessagePack으로 StoryState 저장
pub fn save_state_msgpack<W: Write>(state: &StoryState, writer: W) -> Result<(), CoreError> {
    let mut serializer = Serializer::new(writer);
    state.serialize(&mut serializer).map_err(|e| {
        CoreError::SerializationError(format!("MessagePack serialization failed: {}", e))
    })
}

/// MessagePack에서 StoryState 로드
pub fn load_state_msgpack<R: Read>(reader: R) -> Result<StoryState, CoreError> {
    let mut deserializer = Deserializer::new(reader);
    StoryState::deserialize(&mut deserializer).map_err(|e| {
        CoreError::DeserializationError(format!("MessagePack deserialization failed: {}", e))
    })
}

/// StoryEvent를 MessagePack으로 직렬화
pub fn serialize_event(event: &StoryEvent) -> Result<Vec<u8>, CoreError> {
    rmp_serde::to_vec(event)
        .map_err(|e| CoreError::SerializationError(format!("Event serialization failed: {}", e)))
}

/// MessagePack에서 StoryEvent 역직렬화
pub fn deserialize_event(data: &[u8]) -> Result<StoryEvent, CoreError> {
    rmp_serde::from_slice(data).map_err(|e| {
        CoreError::DeserializationError(format!("Event deserialization failed: {}", e))
    })
}

/// 배치 이벤트 직렬화
pub fn serialize_events_batch(events: &[StoryEvent]) -> Result<Vec<u8>, CoreError> {
    rmp_serde::to_vec(events).map_err(|e| {
        CoreError::SerializationError(format!("Events batch serialization failed: {}", e))
    })
}

/// 배치 이벤트 역직렬화
pub fn deserialize_events_batch(data: &[u8]) -> Result<Vec<StoryEvent>, CoreError> {
    rmp_serde::from_slice(data).map_err(|e| {
        CoreError::DeserializationError(format!("Events batch deserialization failed: {}", e))
    })
}

/// 압축 옵션과 함께 저장
pub struct CompressedSave {
    pub version: u32,
    pub timestamp: i64,
    pub compressed_data: Vec<u8>,
}

impl CompressedSave {
    pub fn create(state: &StoryState) -> Result<Self, CoreError> {
        let data =
            rmp_serde::to_vec(state).map_err(|e| CoreError::SerializationError(e.to_string()))?;

        Ok(Self {
            version: 1,
            timestamp: chrono::Utc::now().timestamp(),
            compressed_data: data, // 추후 zstd 압축 추가 가능
        })
    }

    pub fn extract(&self) -> Result<StoryState, CoreError> {
        rmp_serde::from_slice(&self.compressed_data)
            .map_err(|e| CoreError::DeserializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_serialization_roundtrip() {
        let mut state = StoryState::default();
        state.current_week = 10;
        state.current_route = StoryRoute::Elite;

        // Serialize
        let mut buffer = Vec::new();
        save_state_msgpack(&state, &mut buffer).unwrap();

        // Deserialize
        let loaded = load_state_msgpack(&buffer[..]).unwrap();

        assert_eq!(loaded.current_week, 10);
        assert_eq!(loaded.current_route, StoryRoute::Elite);
    }

    #[test]
    fn test_event_serialization() {
        let event = StoryEvent {
            id: "test_event".to_string(),
            event_type: StoryEventType::Fixed,
            title: "Test Event".to_string(),
            description: "A test event".to_string(),
            choices: vec![],
            conditions: vec![],
            week_range: Some((5, 5)),
            priority: EventPriority::Normal,
            tags: vec!["test".to_string()],
        };

        let serialized = serialize_event(&event).unwrap();
        let deserialized = deserialize_event(&serialized).unwrap();

        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.title, event.title);
    }

    #[test]
    fn test_compressed_save() {
        let state = StoryState::default();
        let compressed = CompressedSave::create(&state).unwrap();

        assert_eq!(compressed.version, 1);
        assert!(!compressed.compressed_data.is_empty());

        let extracted = compressed.extract().unwrap();
        assert_eq!(extracted.current_week, state.current_week);
    }
}

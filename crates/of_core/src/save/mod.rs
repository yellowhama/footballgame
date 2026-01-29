// Save/Load System for OpenFootball
// MessagePack + LZ4 compression with versioning and integrity checks

pub mod error;
pub mod format;
pub mod manager;
pub mod migration;

pub use error::SaveError;
pub use format::{
    decompress_and_deserialize, serialize_and_compress, GameProgress, GameSave, GameSettings,
    MatchRecord, MatchResult,
};
pub use manager::SaveManager;
pub use migration::migrate_save;

pub const SAVE_VERSION: u32 = 1;
pub const SETTINGS_VERSION: u32 = 1;

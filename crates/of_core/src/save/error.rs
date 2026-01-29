use thiserror::Error;

#[derive(Error, Debug)]
pub enum SaveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] rmp_serde::encode::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] rmp_serde::decode::Error),

    #[error("Compression error")]
    Compression,

    #[error("Decompression error")]
    Decompression,

    #[error("Corrupted data")]
    Corrupted,

    #[error("Version mismatch: found {found}, expected {expected}")]
    VersionMismatch { found: u32, expected: u32 },

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid save slot: {slot}")]
    InvalidSlot { slot: i64 },

    #[error("Save data too large: {size} bytes")]
    DataTooLarge { size: usize },
}

impl SaveError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            SaveError::Io(_) => true,
            SaveError::FileNotFound { .. } => true,
            SaveError::InvalidSlot { .. } => false,
            SaveError::Corrupted => false,
            SaveError::ChecksumMismatch => false,
            SaveError::VersionMismatch { .. } => true, // Can try migration
            _ => false,
        }
    }
}

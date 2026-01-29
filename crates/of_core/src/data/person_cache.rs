//! Person cache loader for UID-based match input (MatchRequest v2).
//!
//! Source artifact: `data/exports/cache_players.v5.msgpack.lz4`
//! Format: LZ4 (size-prepended) + MessagePack(serde) of `PersonIndex`.

use crate::models::Person;
use lz4_flex::decompress_size_prepended;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

/// Canonical env var for overriding the person cache path.
pub const PERSON_CACHE_ENV: &str = "OF_PERSON_CACHE_PATH";

/// Legacy alias used by some tooling/docs.
pub const PERSON_CACHE_ENV_ALIAS: &str = "PLAYER_CACHE_PATH";

/// Default relative path used when `OF_PERSON_CACHE_PATH` is not set.
pub const DEFAULT_PERSON_CACHE_REL_PATH: &str = "data/exports/cache_players.v5.msgpack.lz4";

#[derive(Debug, Clone, Deserialize)]
pub struct PersonIndex {
    pub players: HashMap<u32, Person>,
    pub count: u32,
    pub schema_version: String,
}

static PERSON_INDEX: OnceCell<PersonIndex> = OnceCell::new();

#[cfg(feature = "embedded_players")]
const EMBEDDED_PERSON_CACHE_LZ4: &[u8] =
    include_bytes!("../../../../data/exports/cache_players.v5.msgpack.lz4");

fn resolve_cache_path() -> Option<PathBuf> {
    for name in [PERSON_CACHE_ENV, PERSON_CACHE_ENV_ALIAS] {
        if let Ok(path) = env::var(name) {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
    }
    Some(PathBuf::from(DEFAULT_PERSON_CACHE_REL_PATH))
}

fn load_index_from_lz4_bytes(lz4_bytes: &[u8]) -> Result<PersonIndex, String> {
    // Historically this cache has been stored as:
    // - size-prepended LZ4 bytes containing MessagePack(PersonIndex)
    // Some exports (or local workflows) may write the MessagePack payload
    // without LZ4. Prefer the simplest successful decode.
    if let Ok(index) = rmp_serde::from_slice::<PersonIndex>(lz4_bytes) {
        return Ok(index);
    }

    let msgpack_bytes =
        decompress_size_prepended(lz4_bytes).map_err(|e| format!("LZ4 decompress failed: {e}"))?;

    rmp_serde::from_slice::<PersonIndex>(&msgpack_bytes)
        .map_err(|e| format!("MessagePack deserialize failed: {e}"))
}

fn load_index_from_path(path: &Path) -> Result<PersonIndex, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read person cache at '{}': {e}", path.display()))?;
    load_index_from_lz4_bytes(&bytes)
}

/// Load (or return cached) PersonIndex.
///
/// Resolution order:
/// 1) `OF_PERSON_CACHE_PATH` if set (canonical)
/// 1b) `PLAYER_CACHE_PATH` if set (legacy alias)
/// 2) `data/exports/cache_players.v5.msgpack.lz4` (relative)
/// 3) (optional) embedded cache if `embedded_players` feature is enabled
pub fn get_person_index() -> Result<&'static PersonIndex, String> {
    PERSON_INDEX.get_or_try_init(|| {
        let path = resolve_cache_path().ok_or_else(|| "No cache path resolved".to_string())?;
        match load_index_from_path(&path) {
            Ok(index) => Ok(index),
            Err(e) => {
                #[cfg(feature = "embedded_players")]
                {
                    let embedded = load_index_from_lz4_bytes(EMBEDDED_PERSON_CACHE_LZ4)?;
                    return Ok(embedded);
                }
                #[cfg(not(feature = "embedded_players"))]
                {
                    Err(e)
                }
            }
        }
    })
}

/// Resolve a single CSV `Person` by uid (u32).
pub fn get_person_by_uid(uid: u32) -> Result<Option<&'static Person>, String> {
    Ok(get_person_index()?.players.get(&uid))
}

/// Parse cache UID strings.
///
/// Accepted forms:
/// - `csv:<u32>`
/// - `csv_<u32>`
/// - `<u32>` (numeric string)
pub fn parse_person_uid(input: &str) -> Result<u32, ParseIntError> {
    let s = input.trim();
    if let Some(rest) = s.strip_prefix("csv:") {
        return rest.trim().parse::<u32>();
    }
    if let Some(rest) = s.strip_prefix("csv_") {
        return rest.trim().parse::<u32>();
    }
    s.parse::<u32>()
}

/// Resolve a Player UID string into a `Person`.
///
/// Accepted forms (v2.1):
/// - `csv:<u32>` → Person from the shipped cache
/// - `csv_<u32>` → Person from the shipped cache
/// - `<u32>`     → Person from the shipped cache (numeric string)
pub fn resolve_person_by_player_uid(player_uid: &str) -> Result<&'static Person, String> {
    let uid_num: u32 = parse_person_uid(player_uid).map_err(|_| {
        format!("Unsupported player_uid (supports csv:<u32>, csv_<u32>, or <u32>): {player_uid}")
    })?;

    get_person_by_uid(uid_num)?.ok_or_else(|| {
        format!("Player UID not found in person cache (uid={} from {}):", uid_num, player_uid)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_person_uid_accepts_all_supported_forms() {
        assert_eq!(parse_person_uid("csv:123").ok(), Some(123));
        assert_eq!(parse_person_uid("csv_123").ok(), Some(123));
        assert_eq!(parse_person_uid("123").ok(), Some(123));
        assert_eq!(parse_person_uid("  csv: 42 ").ok(), Some(42));
        assert!(parse_person_uid("csv:abc").is_err());
        assert!(parse_person_uid("abc").is_err());
        assert!(parse_person_uid("").is_err());
    }
}

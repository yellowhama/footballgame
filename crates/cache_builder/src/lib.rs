//! Cache Builder Library
//!
//! JSON → MessagePack → LZ4 압축 → SHA256 체크섬 생성
//! CSV (Person data) → Binary cache pipeline

pub mod person_cache;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

// Re-export person_cache types
pub use person_cache::{build_person_cache, load_person_cache, ParseStats, PersonIndex};

// Embedded data support
pub use person_cache::has_embedded_player_cache;
#[cfg(feature = "embedded_players")]
pub use person_cache::load_person_cache_embedded;

/// 캐시 메타데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// 스키마 버전 (예: "v3")
    pub schema_version: String,
    /// SHA256 체크섬 (hex 문자열)
    pub checksum: String,
    /// 생성 시각 (RFC3339 형식)
    pub created_at: String,
    /// 원본 파일 크기 (bytes)
    pub original_size: u64,
    /// 압축 후 크기 (bytes)
    pub compressed_size: u64,
    /// 압축률 (압축 후 / 원본)
    pub compression_ratio: f64,
}

/// JSON 파일을 MessagePack+LZ4로 변환하여 캐시 빌드
///
/// # Arguments
///
/// * `input_json` - 입력 JSON 파일 경로
/// * `output_msgpack_lz4` - 출력 MsgPack+LZ4 파일 경로
/// * `schema_version` - 스키마 버전 문자열
///
/// # Returns
///
/// 생성된 캐시의 메타데이터
pub fn build_cache(
    input_json: &Path,
    output_msgpack_lz4: &Path,
    schema_version: &str,
) -> Result<CacheMetadata> {
    // 1. JSON 파일 읽기
    let json_str = fs::read_to_string(input_json)
        .with_context(|| format!("Failed to read JSON file: {}", input_json.display()))?;

    let original_size = json_str.len() as u64;

    // 2. JSON → serde_json::Value 파싱
    let value: serde_json::Value =
        serde_json::from_str(&json_str).context("Failed to parse JSON")?;

    // 3. MessagePack 직렬화
    let msgpack_bytes = rmp_serde::to_vec(&value).context("Failed to serialize to MessagePack")?;

    // 4. LZ4 압축 (크기 정보 포함)
    let compressed = lz4_flex::compress_prepend_size(&msgpack_bytes);
    let compressed_size = compressed.len() as u64;

    // 5. SHA256 체크섬 계산
    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let checksum = format!("{:x}", hasher.finalize());

    // 6. 출력 파일 쓰기
    if let Some(parent) = output_msgpack_lz4.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    fs::write(output_msgpack_lz4, &compressed).with_context(|| {
        format!(
            "Failed to write output file: {}",
            output_msgpack_lz4.display()
        )
    })?;

    // 7. 메타데이터 생성
    let compression_ratio = compressed_size as f64 / original_size as f64;

    Ok(CacheMetadata {
        schema_version: schema_version.to_string(),
        checksum,
        created_at: chrono::Utc::now().to_rfc3339(),
        original_size,
        compressed_size,
        compression_ratio,
    })
}

/// 캐시 파일의 무결성 검증
///
/// # Arguments
///
/// * `cache_file` - 캐시 파일 경로
/// * `expected_checksum` - 예상되는 SHA256 체크섬
///
/// # Returns
///
/// 체크섬 일치 여부
pub fn verify_cache(cache_file: &Path, expected_checksum: &str) -> Result<bool> {
    let bytes = fs::read(cache_file)
        .with_context(|| format!("Failed to read cache file: {}", cache_file.display()))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());

    Ok(actual == expected_checksum)
}

/// 캐시 파일을 압축 해제하여 MessagePack 디코딩
///
/// # Arguments
///
/// * `cache_file` - 캐시 파일 경로
///
/// # Returns
///
/// 디코딩된 JSON Value
pub fn load_cache(cache_file: &Path) -> Result<serde_json::Value> {
    // 1. 파일 읽기
    let compressed = fs::read(cache_file)
        .with_context(|| format!("Failed to read cache file: {}", cache_file.display()))?;

    // 2. LZ4 압축 해제
    let msgpack_bytes =
        lz4_flex::decompress_size_prepended(&compressed).context("Failed to decompress LZ4")?;

    // 3. MessagePack 디코딩
    let value: serde_json::Value =
        rmp_serde::from_slice(&msgpack_bytes).context("Failed to deserialize MessagePack")?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_build_and_verify_cache() -> Result<()> {
        // 임시 JSON 파일 생성
        let mut temp_json = NamedTempFile::new()?;
        let test_data = serde_json::json!({
            "test": "data",
            "number": 42,
            "array": [1, 2, 3]
        });
        temp_json.write_all(test_data.to_string().as_bytes())?;

        // 임시 출력 파일 경로
        let temp_output = NamedTempFile::new()?;
        let output_path = temp_output.path();

        // 캐시 빌드
        let metadata = build_cache(temp_json.path(), output_path, "v1")?;

        // 검증
        assert_eq!(metadata.schema_version, "v1");
        assert!(metadata.compressed_size < metadata.original_size);
        assert!(verify_cache(output_path, &metadata.checksum)?);

        // 로드 테스트
        let loaded = load_cache(output_path)?;
        assert_eq!(loaded, test_data);

        Ok(())
    }

    #[test]
    fn test_compression_ratio() -> Result<()> {
        let mut temp_json = NamedTempFile::new()?;

        // 반복적인 데이터 (압축이 잘 되는 케이스)
        let large_array: Vec<i32> = (0..1000).map(|i| i % 10).collect();
        let test_data = serde_json::json!({ "data": large_array });
        temp_json.write_all(test_data.to_string().as_bytes())?;

        let temp_output = NamedTempFile::new()?;
        let metadata = build_cache(temp_json.path(), temp_output.path(), "v1")?;

        // 압축률이 50% 미만이어야 함 (압축 효율성 확인)
        assert!(metadata.compression_ratio < 0.5);

        Ok(())
    }
}

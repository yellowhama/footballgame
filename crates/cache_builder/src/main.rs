//! Cache Builder CLI
//!
//! JSON → MessagePack+LZ4 변환 도구
//! CSV → Binary cache 빌더 (선수 데이터)

#[cfg(feature = "cli")]
use anyhow::Result;
#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "cache_builder")]
#[command(about = "Build game caches from JSON or CSV", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Build cache from JSON file
    Json {
        /// Input JSON file path
        #[arg(long)]
        r#in: PathBuf,

        /// Output MsgPack+LZ4 file path
        #[arg(long)]
        out: PathBuf,

        /// Schema version (e.g., "v3")
        #[arg(long)]
        schema_version: String,

        /// Verify cache after building
        #[arg(long, default_value = "false")]
        verify: bool,

        /// Output metadata JSON file
        #[arg(long)]
        metadata: Option<PathBuf>,
    },

    /// Build player cache from CSV file
    Players {
        /// Input CSV file path (players_with_pseudonym.csv)
        #[arg(long)]
        csv: PathBuf,

        /// Output MsgPack+LZ4 file path
        #[arg(long)]
        out: PathBuf,

        /// Schema version (e.g., "v3")
        #[arg(long, default_value = "v3")]
        schema_version: String,

        /// Verify cache after building
        #[arg(long, default_value = "false")]
        verify: bool,

        /// Output metadata JSON file
        #[arg(long)]
        metadata: Option<PathBuf>,
    },
}

#[cfg(feature = "cli")]
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Json {
            r#in,
            out,
            schema_version,
            verify,
            metadata,
        } => {
            println!("🔨 Building JSON cache...");
            println!("   Input:  {}", r#in.display());
            println!("   Output: {}", out.display());
            println!("   Schema: {}", schema_version);

            let meta = cache_builder::build_cache(&r#in, &out, &schema_version)?;

            print_metadata(&meta);

            if verify {
                verify_cache_integrity(&out, &meta.checksum)?;
            }

            if let Some(metadata_path) = metadata {
                save_metadata(&metadata_path, &meta)?;
            }
        }

        Commands::Players {
            csv,
            out,
            schema_version,
            verify,
            metadata,
        } => {
            println!("🔨 Building player cache from CSV...");
            println!("   CSV Input: {}", csv.display());
            println!("   Output:    {}", out.display());
            println!("   Schema:    {}", schema_version);

            let meta = cache_builder::build_person_cache(&csv, &out, &schema_version)?;

            print_metadata(&meta);

            if verify {
                verify_cache_integrity(&out, &meta.checksum)?;
            }

            if let Some(metadata_path) = metadata {
                save_metadata(&metadata_path, &meta)?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn print_metadata(meta: &cache_builder::CacheMetadata) {
    println!("\n✅ Cache built successfully!");
    println!(
        "   Original size:   {} bytes ({:.2} KB)",
        meta.original_size,
        meta.original_size as f64 / 1024.0
    );
    println!(
        "   Compressed size: {} bytes ({:.2} KB)",
        meta.compressed_size,
        meta.compressed_size as f64 / 1024.0
    );
    println!("   Compression:     {:.1}%", meta.compression_ratio * 100.0);
    println!("   Checksum:        {}", meta.checksum);
    println!("   Created:         {}", meta.created_at);
}

#[cfg(feature = "cli")]
fn verify_cache_integrity(cache_path: &std::path::Path, checksum: &str) -> Result<()> {
    println!("\n🔍 Verifying cache integrity...");
    let is_valid = cache_builder::verify_cache(cache_path, checksum)?;

    if is_valid {
        println!("✅ Cache verification passed");
        Ok(())
    } else {
        anyhow::bail!("❌ Cache verification failed - checksum mismatch!")
    }
}

#[cfg(feature = "cli")]
fn save_metadata(path: &PathBuf, meta: &cache_builder::CacheMetadata) -> Result<()> {
    let metadata_json = serde_json::to_string_pretty(meta)?;
    std::fs::write(path, metadata_json)?;
    println!("\n📄 Metadata saved to: {}", path.display());
    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("cache_builder CLI is not available. Enable the 'cli' feature to use it.");
    std::process::exit(1);
}

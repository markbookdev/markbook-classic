use anyhow::{anyhow, Context};
use serde_json::json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MANIFEST_ENTRY: &str = "manifest.json";
const DB_ENTRY: &str = "db/markbook.sqlite3";
const META_WORKSPACE_ENTRY: &str = "meta/workspace.json";
pub const BUNDLE_FORMAT_V2: &str = "markbook-workspace-v2";

#[derive(Debug, Clone)]
pub struct ExportSummary {
    pub bundle_format: String,
    pub entry_count: usize,
}

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub bundle_format_detected: String,
}

pub fn export_workspace_bundle(
    workspace_path: &Path,
    out_path: &Path,
) -> anyhow::Result<ExportSummary> {
    let db_path = workspace_path.join("markbook.sqlite3");
    if !db_path.is_file() {
        return Err(anyhow!(
            "workspace database not found: {}",
            db_path.to_string_lossy()
        ));
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.to_string_lossy()))?;
    }

    let out_file = File::create(out_path).with_context(|| {
        format!(
            "failed to create output file {}",
            out_path.to_string_lossy()
        )
    })?;
    let mut zip = ZipWriter::new(out_file);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);

    let exported_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let manifest = json!({
        "format": BUNDLE_FORMAT_V2,
        "version": 2,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "exportedAt": exported_at,
    });
    zip.start_file(MANIFEST_ENTRY, opts)
        .context("failed to start manifest entry")?;
    zip.write_all(
        serde_json::to_string_pretty(&manifest)
            .context("failed to serialize manifest")?
            .as_bytes(),
    )
    .context("failed to write manifest entry")?;

    zip.start_file(DB_ENTRY, opts)
        .context("failed to start database entry")?;
    let mut db_file = File::open(&db_path)
        .with_context(|| format!("failed to open database {}", db_path.to_string_lossy()))?;
    std::io::copy(&mut db_file, &mut zip).context("failed to write database entry")?;

    let workspace_meta = json!({
        "sourceWorkspace": workspace_path.to_string_lossy(),
    });
    zip.start_file(META_WORKSPACE_ENTRY, opts)
        .context("failed to start workspace metadata entry")?;
    zip.write_all(
        serde_json::to_string_pretty(&workspace_meta)
            .context("failed to serialize workspace metadata")?
            .as_bytes(),
    )
    .context("failed to write workspace metadata entry")?;

    zip.finish().context("failed to finalize zip bundle")?;

    Ok(ExportSummary {
        bundle_format: BUNDLE_FORMAT_V2.to_string(),
        entry_count: 3,
    })
}

pub fn import_workspace_bundle(
    in_path: &Path,
    workspace_path: &Path,
) -> anyhow::Result<ImportSummary> {
    std::fs::create_dir_all(workspace_path).with_context(|| {
        format!(
            "failed to create workspace {}",
            workspace_path.to_string_lossy()
        )
    })?;
    let dst = workspace_path.join("markbook.sqlite3");

    if !is_zip_file(in_path)? {
        std::fs::copy(in_path, &dst).with_context(|| {
            format!(
                "failed to copy legacy sqlite backup from {} to {}",
                in_path.to_string_lossy(),
                dst.to_string_lossy()
            )
        })?;
        return Ok(ImportSummary {
            bundle_format_detected: "legacy-sqlite3".to_string(),
        });
    }

    let in_file = File::open(in_path)
        .with_context(|| format!("failed to open bundle {}", in_path.to_string_lossy()))?;
    let mut archive = ZipArchive::new(in_file).context("invalid zip archive")?;

    let mut manifest_text = String::new();
    archive
        .by_name(MANIFEST_ENTRY)
        .context("bundle missing manifest.json")?
        .read_to_string(&mut manifest_text)
        .context("failed to read manifest.json")?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).context("manifest.json is invalid JSON")?;
    let format = manifest
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if format != BUNDLE_FORMAT_V2 {
        return Err(anyhow!("unsupported bundle format: {}", format));
    }

    let tmp_dst = workspace_path.join("markbook.sqlite3.importing");
    if tmp_dst.exists() {
        let _ = std::fs::remove_file(&tmp_dst);
    }

    let mut db_out = File::create(&tmp_dst).with_context(|| {
        format!(
            "failed to create temp database {}",
            tmp_dst.to_string_lossy()
        )
    })?;
    {
        let mut db_entry = archive
            .by_name(DB_ENTRY)
            .context("bundle missing db/markbook.sqlite3")?;
        std::io::copy(&mut db_entry, &mut db_out).context("failed to extract database entry")?;
    }
    db_out
        .flush()
        .context("failed to flush extracted database")?;

    if dst.exists() {
        std::fs::remove_file(&dst).with_context(|| {
            format!(
                "failed to remove existing database {}",
                dst.to_string_lossy()
            )
        })?;
    }
    std::fs::rename(&tmp_dst, &dst).with_context(|| {
        format!(
            "failed to move extracted database to {}",
            dst.to_string_lossy()
        )
    })?;

    Ok(ImportSummary {
        bundle_format_detected: BUNDLE_FORMAT_V2.to_string(),
    })
}

fn is_zip_file(path: &Path) -> anyhow::Result<bool> {
    let mut f = File::open(path)
        .with_context(|| format!("failed to open input file {}", path.to_string_lossy()))?;
    let mut sig = [0u8; 4];
    let read = f.read(&mut sig).context("failed to read file signature")?;
    if read < 4 {
        return Ok(false);
    }
    Ok(sig == [0x50, 0x4B, 0x03, 0x04])
}

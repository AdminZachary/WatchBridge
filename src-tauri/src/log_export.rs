//! Export the on-disk log files so a report can be attached to an issue.
//!
//! The logs already pass the privacy filter in `logging`, so what is written
//! here is the same content the app itself persisted.
use crate::error::AppError;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Cap the export so a long-running install cannot produce a huge attachment.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    files: usize,
    bytes: u64,
    truncated: bool,
}

fn is_log_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("iloader") && name.ends_with(".log"))
}

// Truncate on a character boundary so the exported text stays valid UTF-8.
fn take_within(text: &str, limit: u64) -> (&str, bool) {
    if text.len() as u64 <= limit {
        return (text, false);
    }
    let mut cut = 0;
    for (index, character) in text.char_indices() {
        if index as u64 >= limit {
            break;
        }
        cut = index + character.len_utf8();
    }
    (&text[..cut], true)
}

#[tauri::command]
pub fn export_logs(app: AppHandle, destination: String) -> Result<ExportReport, AppError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Misc("无法访问应用数据目录。".into()))?
        .join("logs");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .map_err(|_| AppError::Misc("还没有可导出的日志。".into()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_log_file(path))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(AppError::Misc("还没有可导出的日志。".into()));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "# WatchBridge logs\n# exported: {}\n# version: {}\n# os: {}\n",
        chrono::Utc::now().to_rfc3339(),
        app.package_info().version,
        std::env::consts::OS,
    ));

    let mut bytes = 0u64;
    let mut files = 0usize;
    let mut truncated = false;
    for path in &paths {
        let raw = std::fs::read(path).map_err(|error| {
            AppError::Misc(format!("无法读取日志 {}: {error}", path.display()))
        })?;
        let text = String::from_utf8_lossy(&raw).into_owned();
        let (piece, cut) = take_within(&text, MAX_BYTES.saturating_sub(bytes));
        out.push_str(&format!(
            "\n===== {} =====\n",
            path.file_name().and_then(|name| name.to_str()).unwrap_or("log")
        ));
        out.push_str(piece);
        if !piece.ends_with('\n') {
            out.push('\n');
        }
        bytes += piece.len() as u64;
        files += 1;
        if cut {
            truncated = true;
            break;
        }
    }

    std::fs::write(&destination, out)
        .map_err(|error| AppError::Misc(format!("无法写入目标文件: {error}")))?;
    Ok(ExportReport {
        files,
        bytes,
        truncated,
    })
}

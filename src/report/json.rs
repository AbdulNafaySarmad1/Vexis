//! JSON report — the machine-readable artifact.

use super::AnalysisResult;

pub fn to_string(result: &AnalysisResult) -> serde_json::Result<String> {
    serde_json::to_string_pretty(result)
}

pub fn write_to(result: &AnalysisResult, path: &std::path::Path) -> std::io::Result<()> {
    let s = to_string(result).map_err(std::io::Error::other)?;
    std::fs::write(path, s)
}

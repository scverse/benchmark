use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use anyhow::{bail, Result};
use clap_lex::OsStrExt;

use super::asv_config::AsvConfig;

#[derive(Debug, serde::Deserialize)]
struct AsvResult {
    commit_hash: String,
    result_columns: Vec<String>,
    results: HashMap<String, Vec<serde_json::Value>>,
}

pub(super) fn compare(config: &AsvConfig, before: &str, after: &str) -> Result<()> {
    // TODO: use correct machine instead of first
    let Some(machine_dir) = config
        .results_dir
        .read_dir()?
        .filter_map(Result::ok)
        .find(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
        .map(|e| e.path())
    else {
        bail!("no machine dir found");
    };

    let before = find_commit(&machine_dir, before)?;
    let after = find_commit(&machine_dir, after)?;

    Ok(())
}

fn find_commit(machine_dir: &Path, commit: &str) -> Result<AsvResult> {
    for candidate in machine_dir
        .read_dir()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().starts_with(&commit[..8]))
        .map(|e| e.path())
    {
        let mut reader = BufReader::new(File::open(candidate)?);
        let candidate: AsvResult = serde_json::from_reader(&mut reader)?;
        if candidate.commit_hash == commit {
            return Ok(candidate);
        }
    }
    bail!("could not find result for commit {commit} in {machine_dir:?}");
}

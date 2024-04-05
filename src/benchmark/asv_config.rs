use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[serde_inline_default]
#[derive(Deserialize)]
pub(super) struct AsvConfig {
    /// Branches to benchmark by default
    #[serde_inline_default(vec!["master".to_owned()])]
    pub branches: Vec<String>,

    /// Directory to cache the Python environments in.
    #[serde_inline_default("env".into())]
    pub env_dir: PathBuf,

    /// Directory to store raw benchmark results in.
    #[serde_inline_default("results".into())]
    pub results_dir: PathBuf,

    /// Directory to write html tree into.
    #[serde_inline_default("html".into())]
    pub html_dir: PathBuf,
}

impl AsvConfig {
    pub fn from_path(path: &Path) -> Result<Self> {
        let mut file = BufReader::new(File::open(path)?);
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // make paths absolute
        let wd = path
            .parent()
            .ok_or_else(|| anyhow!("path has no parent: {path:?}"))?;
        let mut conf: Self = serde_json5::from_str(&contents)?;
        conf.env_dir = wd.join(&conf.env_dir);
        conf.results_dir = wd.join(&conf.results_dir);
        conf.html_dir = wd.join(&conf.html_dir);
        Ok(conf)
    }

    pub fn from_wd(wd: &Path) -> Result<Self> {
        Self::from_path(&wd.join("asv.conf.json"))
    }
}

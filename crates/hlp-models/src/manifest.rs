// Component layout and GGUF pins adapted from Hen-Local-Translator/hen-local-models/src/lib.rs.
// The list of MLX files is adapted from Hen-Local-Translator/hen-local-init/src/main.rs.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Artifact {
    pub url: String,
    pub mirror: Option<String>,
    pub bytes: u64,
    pub sha256: String,
    pub name: Option<String>,
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub members: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Format {
    #[default]
    Raw,
    Zip,
    Tgz,
    Tbz,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Check {
    Mlx,
    Sherpa,
    File,
    Gguf,
    Tool,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Component {
    pub id: String,
    pub platform: String,
    pub group: String,
    #[serde(rename = "kind")]
    pub _kind: String,
    pub directory: PathBuf,
    pub ready: Check,
    pub files: Vec<Artifact>,
    pub repo: Option<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}
impl Component {
    pub fn total(&self) -> u64 {
        self.files.iter().map(|a| a.bytes).sum()
    }
    pub fn stage(&self, root: &Path) -> PathBuf {
        root.join(".hlp-setup").join(&self.id)
    }
}
pub(crate) static COMPONENTS: LazyLock<Vec<Component>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("components.json")).expect("validated component catalog")
});
pub(crate) fn catalog() -> impl Iterator<Item = &'static Component> {
    COMPONENTS
        .iter()
        .filter(|c| c.platform == "both" || c.platform == std::env::consts::OS)
}
pub(crate) fn component(id: &str) -> Option<&'static Component> {
    catalog().find(|c| c.id == id)
}

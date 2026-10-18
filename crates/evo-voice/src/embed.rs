//! Production text embedders via fastembed, on-device.
//!
//! This is the concrete [`evo_semantic::Embedder`] that evo-retrieval's
//! semantic resolution uses in production. Initialization reads only installed
//! local files; it never constructs a Hugging Face client, reads authentication
//! tokens, or downloads missing assets. Provision the model separately.
//!
//! The default is all-MiniLM-L6-v2, which produces 384-dimensional vectors.
//! An explicit multilingual E5-small profile is also supported. It uses the
//! model's asymmetric `passage:`/`query:` instructions and remains strictly
//! local-only.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use evo_semantic::{EmbedError, Embedder, EmbeddingRole};
use fastembed::{
    InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel,
};

/// all-MiniLM-L6-v2 embedding dimension.
pub const EMBEDDING_DIM: usize = 384;
/// multilingual-e5-small embedding dimension.
pub const MULTILINGUAL_E5_SMALL_EMBEDDING_DIM: usize = 384;

// The exact non-quantized AllMiniLML6V2 export selected by fastembed 7.1.0.
const CACHE_MODEL_DIR: &str = "models--Qdrant--all-MiniLM-L6-v2-onnx";
const MODEL_FILES: [&str; 5] = [
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];
// fastembed 7.1.0 selects intfloat/multilingual-e5-small and its ONNX file
// lives under `onnx/` in the model repository.
const E5_CACHE_MODEL_DIR: &str = "models--intfloat--multilingual-e5-small";
const E5_MODEL_FILES: [&str; 5] = [
    "onnx/model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

const PROFILE_ENV: &str = "EVO_EMBEDDING_PROFILE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmbeddingProfile {
    AllMiniLML6V2,
    MultilingualE5Small,
}

impl EmbeddingProfile {
    fn parse(name: &str) -> Result<Self, String> {
        let normalized = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        match normalized.as_str() {
            "all-minilm-l6-v2" | "allminlml6v2" | "minilm" | "default" => Ok(Self::AllMiniLML6V2),
            "multilingual-e5-small" | "multilinguale5small" | "e5-small" => {
                Ok(Self::MultilingualE5Small)
            }
            _ => Err(format!(
                "unsupported embedding profile {name:?}; supported profiles are \
                 all-MiniLM-L6-v2 and multilingual-e5-small"
            )),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::AllMiniLML6V2 => "all-MiniLM-L6-v2",
            Self::MultilingualE5Small => "multilingual-e5-small",
        }
    }

    const fn dimension(self) -> usize {
        match self {
            Self::AllMiniLML6V2 => EMBEDDING_DIM,
            Self::MultilingualE5Small => MULTILINGUAL_E5_SMALL_EMBEDDING_DIM,
        }
    }

    const fn cache_model_dir(self) -> &'static str {
        match self {
            Self::AllMiniLML6V2 => CACHE_MODEL_DIR,
            Self::MultilingualE5Small => E5_CACHE_MODEL_DIR,
        }
    }

    const fn model_files(self) -> &'static [&'static str; 5] {
        match self {
            Self::AllMiniLML6V2 => &MODEL_FILES,
            Self::MultilingualE5Small => &E5_MODEL_FILES,
        }
    }

    const fn input_prefix(self, role: EmbeddingRole) -> &'static str {
        match (self, role) {
            (Self::AllMiniLML6V2, _) => "",
            (Self::MultilingualE5Small, EmbeddingRole::Document) => "passage: ",
            (Self::MultilingualE5Small, EmbeddingRole::Query) => "query: ",
        }
    }

    fn prepare_input(self, role: EmbeddingRole, text: &str) -> Result<String, EmbedError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(EmbedError::Refused("empty text".to_string()));
        }
        Ok(format!("{}{}", self.input_prefix(role), trimmed))
    }

    fn validate_embedding(self, vector: &[f32]) -> Result<(), EmbedError> {
        if vector.len() != self.dimension() {
            return Err(EmbedError::DimensionMismatch {
                expected: self.dimension(),
                got: vector.len(),
            });
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(EmbedError::Refused(format!(
                "{} returned a nonfinite embedding",
                self.name()
            )));
        }
        if vector.iter().all(|value| *value == 0.0) {
            return Err(EmbedError::Refused(format!(
                "{} returned a zero-norm embedding",
                self.name()
            )));
        }
        Ok(())
    }
}

/// The production embedder, backed by fastembed's all-MiniLM-L6-v2.
///
/// Loads installed model files on construction. Missing or invalid files are
/// errors; there is no remote fallback. Wrap in an Arc to share the handle.
///
/// # Example
///
/// ```no_run
/// use evo_voice::FastEmbedEmbedder;
/// use evo_semantic::Embedder;
///
/// let embedder = FastEmbedEmbedder::load().expect("model");
/// let vec = embedder.embed("i was fixing the technology of evo").expect("embed");
/// assert_eq!(vec.len(), 384);
/// ```
pub struct FastEmbedEmbedder {
    model: Mutex<TextEmbedding>,
    profile: EmbeddingProfile,
}

impl FastEmbedEmbedder {
    /// Reports the selected offline assets without initializing ONNX.
    ///
    /// Selection is identical to [`Self::load`]. This checks presence/readability,
    /// not hashes or inference. For profile-specific integrity guidance, see
    /// `docs/model-local.md`.
    pub fn doctor() -> Result<String, String> {
        let profile = profile_from_env(|name| std::env::var_os(name))?;
        let dir = match local_source_for_profile(|name| std::env::var_os(name), profile)? {
            LocalSource::ModelDir(dir) => dir,
            LocalSource::CacheDir(dir) => cached_model_dir_for_profile(&dir, profile)?,
        };
        Self::doctor_from_model_dir_with_profile(dir, profile)
    }

    /// Checks a flat local directory or exact cache snapshot without networking.
    /// Like [`Self::load_from_model_dir`], environment variables are not used.
    pub fn doctor_from_model_dir(model_dir: impl Into<PathBuf>) -> Result<String, String> {
        Self::doctor_from_model_dir_with_profile(model_dir.into(), EmbeddingProfile::AllMiniLML6V2)
    }

    /// Checks a local model directory for a named profile without initializing
    /// ONNX. The profile must be one of `all-MiniLM-L6-v2` or
    /// `multilingual-e5-small`.
    pub fn doctor_from_model_dir_for_profile(
        model_dir: impl Into<PathBuf>,
        profile: &str,
    ) -> Result<String, String> {
        let profile = EmbeddingProfile::parse(profile)?;
        Self::doctor_from_model_dir_with_profile(model_dir.into(), profile)
    }

    fn doctor_from_model_dir_with_profile(
        model_dir: PathBuf,
        profile: EmbeddingProfile,
    ) -> Result<String, String> {
        if model_dir.as_os_str().is_empty() {
            return Err(offline_error(profile, "model directory is empty"));
        }
        let mut total_bytes = 0;
        for name in profile.model_files() {
            let path = model_dir.join(name);
            total_bytes += asset_metadata(profile, &path)?.len();
            fs::File::open(&path).map_err(|e| {
                offline_error(
                    profile,
                    format!("cannot read required asset '{}': {e}", path.display()),
                )
            })?;
        }
        Ok(format!(
            "offline {}: five readable nonempty local assets in '{}' ({total_bytes} bytes); \
             presence only, hashes and inference not checked; verify local asset integrity in docs/model-local.md",
            profile.name(),
            model_dir.display()
        ))
    }

    /// Loads the installed all-MiniLM-L6-v2 model, strictly offline.
    ///
    /// If `EVO_EMBEDDING_MODEL_DIR` is set, it names the exact directory described
    /// by [`Self::load_from_model_dir`]. Otherwise the cache root is `HF_HOME`,
    /// then `FASTEMBED_CACHE_DIR`, then `.fastembed_cache` in the current working
    /// directory (the actual fastembed 7.1.0 defaults). `HF_HOME` is used directly,
    /// without appending `hub`, matching fastembed's previous initializer.
    ///
    /// Relative paths are relative to the current working directory; no tilde
    /// expansion is performed. A configured empty, missing or invalid location
    /// fails without trying lower-priority locations. See
    /// [`Self::load_with_cache_dir`] for the cache layout.
    pub fn load() -> Result<Self, String> {
        let profile = profile_from_env(|name| std::env::var_os(name))?;
        Self::load_from_environment_for_profile(profile)
    }

    /// Loads a named local profile using the configured local directory/cache.
    /// This is equivalent to setting `EVO_EMBEDDING_PROFILE` for one load and
    /// never enables fastembed's hub-backed initializer.
    pub fn load_for_profile(profile: &str) -> Result<Self, String> {
        let profile = EmbeddingProfile::parse(profile)?;
        Self::load_from_environment_for_profile(profile)
    }

    fn load_from_environment_for_profile(profile: EmbeddingProfile) -> Result<Self, String> {
        match local_source_for_profile(|name| std::env::var_os(name), profile)? {
            LocalSource::ModelDir(dir) => Self::load_from_model_dir_with_profile(dir, profile),
            LocalSource::CacheDir(dir) => Self::load_with_cache_dir_with_profile(dir, profile),
        }
    }

    /// Loads an existing fastembed/Hugging Face cache, strictly offline.
    ///
    /// Reads `<cache_dir>/models--Qdrant--all-MiniLM-L6-v2-onnx/refs/main`, then
    /// loads the five files listed in [`Self::load_from_model_dir`] from that
    /// repository's `snapshots/<revision>/` directory. Ordinary files and cache
    /// symlinks to local blobs are supported. The ref must name a single snapshot
    /// directory; other revisions are never guessed or used as fallbacks.
    ///
    /// This explicit path takes precedence over all environment variables.
    /// For a flat directory or a pinned snapshot use [`Self::load_from_model_dir`].
    pub fn load_with_cache_dir(cache_dir: impl Into<PathBuf>) -> Result<Self, String> {
        Self::load_with_cache_dir_for_profile(cache_dir, "all-MiniLM-L6-v2")
    }

    /// Loads an existing cache for a named profile, strictly offline.
    pub fn load_with_cache_dir_for_profile(
        cache_dir: impl Into<PathBuf>,
        profile: &str,
    ) -> Result<Self, String> {
        let profile = EmbeddingProfile::parse(profile)?;
        Self::load_with_cache_dir_with_profile(cache_dir, profile)
    }

    fn load_with_cache_dir_with_profile(
        cache_dir: impl Into<PathBuf>,
        profile: EmbeddingProfile,
    ) -> Result<Self, String> {
        Self::load_from_model_dir_with_profile(
            cached_model_dir_for_profile(&cache_dir.into(), profile)?,
            profile,
        )
    }

    /// Loads a flat local model directory or an exact cache snapshot, offline.
    ///
    /// The directory must contain `model.onnx`, `tokenizer.json`, `config.json`,
    /// `special_tokens_map.json`, and `tokenizer_config.json` from fastembed's
    /// `Qdrant/all-MiniLM-L6-v2-onnx` export. This is the non-quantized,
    /// 384-dimensional model with mean pooling; other models/exports are not
    /// interchangeable. External ONNX weight files are not part of this export.
    /// No environment variables override this path, and no cache is searched.
    pub fn load_from_model_dir(model_dir: impl Into<PathBuf>) -> Result<Self, String> {
        Self::load_from_model_dir_for_profile(model_dir, "all-MiniLM-L6-v2")
    }

    /// Loads a named profile from a flat local directory or exact cache
    /// snapshot, offline. The directory must contain the profile's exact
    /// fastembed 7.1 files; no other model is guessed.
    pub fn load_from_model_dir_for_profile(
        model_dir: impl Into<PathBuf>,
        profile: &str,
    ) -> Result<Self, String> {
        let profile = EmbeddingProfile::parse(profile)?;
        Self::load_from_model_dir_with_profile(model_dir, profile)
    }

    fn load_from_model_dir_with_profile(
        model_dir: impl Into<PathBuf>,
        profile: EmbeddingProfile,
    ) -> Result<Self, String> {
        let model_dir = model_dir.into();
        let assets = read_local_model_for_profile(&model_dir, profile)?;
        // Unlike try_new(TextInitOptions), this path only creates a local ONNX
        // session and tokenizer from bytes; it has no hub/cache/client access.
        let model = TextEmbedding::try_new_from_user_defined(assets, InitOptionsUserDefined::new())
            .map_err(|e| {
                offline_error(
                    profile,
                    format!(
                        "invalid {} or tokenizer/config assets in '{}': {e}",
                        profile.model_files()[0],
                        model_dir.display()
                    ),
                )
            })?;
        Ok(Self {
            model: Mutex::new(model),
            profile,
        })
    }

    /// The stable name of the locally selected model profile.
    pub fn profile_name(&self) -> &'static str {
        self.profile.name()
    }

    /// The vector width guaranteed by the selected model profile.
    pub fn embedding_dimension(&self) -> usize {
        self.profile.dimension()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LocalSource {
    ModelDir(PathBuf),
    CacheDir(PathBuf),
}

#[cfg(test)]
fn local_source(mut env: impl FnMut(&str) -> Option<OsString>) -> Result<LocalSource, String> {
    local_source_for_profile(&mut env, EmbeddingProfile::AllMiniLML6V2)
}

fn local_source_for_profile(
    mut env: impl FnMut(&str) -> Option<OsString>,
    profile: EmbeddingProfile,
) -> Result<LocalSource, String> {
    for name in ["EVO_EMBEDDING_MODEL_DIR", "HF_HOME", "FASTEMBED_CACHE_DIR"] {
        if let Some(value) = env(name) {
            if value.is_empty() {
                return Err(offline_error(
                    profile,
                    format!("{name} is set to an empty path"),
                ));
            }
            return Ok(if name == "EVO_EMBEDDING_MODEL_DIR" {
                LocalSource::ModelDir(value.into())
            } else {
                LocalSource::CacheDir(value.into())
            });
        }
    }
    // Stable per-user install location, checked with lower precedence than
    // all environment variable options but before the cwd-relative cache.
    // The LaunchAgent runs with cwd = / : a cwd-relative cache can never be
    // found there, so headless voice triggers failed honestly with
    // "voice model unavailable" even when the assets sat on disk.
    if let Some(home) = std::env::var_os("HOME") {
        let installed =
            std::path::PathBuf::from(home).join("Library/Application Support/Evo/embedding");
        if installed.is_dir() {
            return Ok(LocalSource::ModelDir(installed));
        }
    }
    Ok(LocalSource::CacheDir(".fastembed_cache".into()))
}

fn profile_from_env(
    mut env: impl FnMut(&str) -> Option<OsString>,
) -> Result<EmbeddingProfile, String> {
    match env(PROFILE_ENV) {
        None => Ok(EmbeddingProfile::AllMiniLML6V2),
        Some(value) if value.is_empty() => Err(format!(
            "offline embedding: {PROFILE_ENV} is set to an empty profile; supported profiles are \
             all-MiniLM-L6-v2 and multilingual-e5-small; no download or remote fallback attempted"
        )),
        Some(value) => EmbeddingProfile::parse(&value.to_string_lossy()),
    }
}

fn offline_error(profile: EmbeddingProfile, detail: impl std::fmt::Display) -> String {
    let provision = match profile {
        EmbeddingProfile::AllMiniLML6V2 => {
            "provision explicitly with python3 scripts/model/minilm.py fetch (docs/model-local.md)"
        }
        EmbeddingProfile::MultilingualE5Small => {
            "provision explicitly with the multilingual-e5-small local assets (docs/model-local.md)"
        }
    };
    format!(
        "offline {}: {detail}; no download or remote fallback attempted; {provision}",
        profile.name(),
    )
}

#[cfg(test)]
fn cached_model_dir(cache_dir: &Path) -> Result<PathBuf, String> {
    cached_model_dir_for_profile(cache_dir, EmbeddingProfile::AllMiniLML6V2)
}

fn cached_model_dir_for_profile(
    cache_dir: &Path,
    profile: EmbeddingProfile,
) -> Result<PathBuf, String> {
    if cache_dir.as_os_str().is_empty() {
        return Err(offline_error(profile, "cache directory is empty"));
    }
    let repo = cache_dir.join(profile.cache_model_dir());
    let ref_path = repo.join("refs/main");
    let revision = fs::read_to_string(&ref_path).map_err(|e| {
        offline_error(
            profile,
            format!("cannot read cache ref '{}': {e}", ref_path.display()),
        )
    })?;
    let revision = revision.trim();
    // Hugging Face writes commit hashes here. Reject traversal, absolute paths,
    // empty refs and malformed content instead of searching for another model.
    if revision.is_empty() || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(offline_error(
            profile,
            format!(
                "invalid cache ref '{}': expected a hexadecimal snapshot revision",
                ref_path.display()
            ),
        ));
    }
    Ok(repo.join("snapshots").join(revision))
}

fn asset_metadata(profile: EmbeddingProfile, path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::metadata(path).map_err(|e| {
        offline_error(
            profile,
            format!("cannot read required asset '{}': {e}", path.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(offline_error(
            profile,
            format!("required asset '{}' is not a regular file", path.display()),
        ));
    }
    if metadata.len() == 0 {
        return Err(offline_error(
            profile,
            format!("required asset '{}' is empty", path.display()),
        ));
    }
    Ok(metadata)
}

fn read_asset(profile: EmbeddingProfile, model_dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let path = model_dir.join(name);
    asset_metadata(profile, &path)?;
    let bytes = fs::read(&path).map_err(|e| {
        offline_error(
            profile,
            format!("cannot read required asset '{}': {e}", path.display()),
        )
    })?;
    if bytes.is_empty() {
        return Err(offline_error(
            profile,
            format!("required asset '{}' is empty", path.display()),
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
fn read_local_model(model_dir: &Path) -> Result<UserDefinedEmbeddingModel, String> {
    read_local_model_for_profile(model_dir, EmbeddingProfile::AllMiniLML6V2)
}

fn read_local_model_for_profile(
    model_dir: &Path,
    profile: EmbeddingProfile,
) -> Result<UserDefinedEmbeddingModel, String> {
    if model_dir.as_os_str().is_empty() {
        return Err(offline_error(profile, "model directory is empty"));
    }
    let files = profile.model_files();
    let onnx = read_asset(profile, model_dir, files[0])?;
    let tokenizer = TokenizerFiles {
        tokenizer_file: read_asset(profile, model_dir, files[1])?,
        config_file: read_asset(profile, model_dir, files[2])?,
        special_tokens_map_file: read_asset(profile, model_dir, files[3])?,
        tokenizer_config_file: read_asset(profile, model_dir, files[4])?,
    };
    Ok(UserDefinedEmbeddingModel::new(onnx, tokenizer).with_pooling(Pooling::Mean))
}

impl Embedder for FastEmbedEmbedder {
    fn allows_similarity_restore(&self) -> bool {
        // RETRIEVAL_FLOOR/MARGIN were measured with MiniLM. E5 has a different
        // score distribution; similarity can suggest, but cannot authorize an
        // automatic restore until held-out local calibration is complete.
        self.profile == EmbeddingProfile::AllMiniLML6V2
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed_with_role(EmbeddingRole::Document, text)
    }

    fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed_with_role(EmbeddingRole::Document, text)
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed_with_role(EmbeddingRole::Query, text)
    }
}

impl FastEmbedEmbedder {
    fn embed_with_role(&self, role: EmbeddingRole, text: &str) -> Result<Vec<f32>, EmbedError> {
        let input = self.profile.prepare_input(role, text)?;
        let mut model = self
            .model
            .lock()
            .map_err(|e| EmbedError::Refused(format!("embedder lock poisoned: {e}")))?;
        let mut embeddings = model
            .embed(vec![input], None)
            .map_err(|e| EmbedError::Refused(format!("embedding failed: {e}")))?;
        let vec = embeddings
            .pop()
            .ok_or_else(|| EmbedError::Refused("model returned no embedding".to_string()))?;
        self.profile.validate_embedding(&vec)?;
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastembed::{EmbeddingModel, QuantizationMode};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "evo-embed-offline-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    // Deliberately invalid payloads test file plumbing and rejection, never
    // successful model inference. No real or fabricated model weights needed.
    fn write_asset_markers(dir: &Path) {
        write_profile_asset_markers(dir, EmbeddingProfile::AllMiniLML6V2);
    }

    fn write_profile_asset_markers(dir: &Path, profile: EmbeddingProfile) {
        fs::create_dir_all(dir).unwrap();
        for file in profile.model_files() {
            let path = dir.join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, format!("invalid fixture: {file}")).unwrap();
        }
    }

    fn write_cache_ref(root: &Path, revision: &str) -> PathBuf {
        let repo = root.join(CACHE_MODEL_DIR);
        fs::create_dir_all(repo.join("refs")).unwrap();
        fs::write(repo.join("refs/main"), revision).unwrap();
        repo.join("snapshots").join(revision.trim())
    }

    fn load_error(result: Result<FastEmbedEmbedder, String>) -> String {
        result.err().expect("invalid local fixture must fail")
    }

    #[test]
    fn profile_selection_is_explicit_and_rejects_unknown_models() {
        assert_eq!(
            EmbeddingProfile::parse("multilingual-e5-small").unwrap(),
            EmbeddingProfile::MultilingualE5Small
        );
        assert_eq!(
            EmbeddingProfile::parse("all_MiniLM_L6_v2").unwrap(),
            EmbeddingProfile::AllMiniLML6V2
        );
        assert_eq!(
            profile_from_env(|name| { (name == PROFILE_ENV).then(|| OsString::from("e5-small")) })
                .unwrap(),
            EmbeddingProfile::MultilingualE5Small
        );
        let err = EmbeddingProfile::parse("remote-model").unwrap_err();
        assert!(err.contains("unsupported embedding profile"), "{err}");
        assert!(
            !err.contains("no download"),
            "profile parsing is local-only"
        );
    }

    #[test]
    fn multilingual_e5_profile_has_expected_dimension_and_role_prefixes() {
        let profile = EmbeddingProfile::MultilingualE5Small;
        let info = TextEmbedding::get_model_info(&EmbeddingModel::MultilingualE5Small).unwrap();
        assert_eq!(profile.dimension(), 384);
        assert_eq!(profile.dimension(), info.dim);
        assert_eq!(info.model_code, "intfloat/multilingual-e5-small");
        assert_eq!(info.model_file, "onnx/model.onnx");
        assert_eq!(
            profile
                .prepare_input(EmbeddingRole::Document, "  texto multilingual ")
                .unwrap(),
            "passage: texto multilingual"
        );
        assert_eq!(
            profile
                .prepare_input(EmbeddingRole::Query, "  texto multilingual ")
                .unwrap(),
            "query: texto multilingual"
        );
        assert_ne!(
            profile
                .prepare_input(EmbeddingRole::Document, "text")
                .unwrap(),
            profile.prepare_input(EmbeddingRole::Query, "text").unwrap()
        );
    }

    #[test]
    fn missing_multilingual_assets_refuse_without_network_or_cache_mutation() {
        let root = FixtureDir::new();
        let err = load_error(FastEmbedEmbedder::load_with_cache_dir_for_profile(
            &root.0,
            "multilingual-e5-small",
        ));
        assert!(err.contains(E5_CACHE_MODEL_DIR), "{err}");
        assert!(err.contains("multilingual-e5-small"), "{err}");
        assert!(err.contains("no download or remote fallback"), "{err}");
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
    }

    #[test]
    fn multilingual_local_asset_layout_is_selected_without_fallback() {
        let root = FixtureDir::new();
        write_profile_asset_markers(&root.0, EmbeddingProfile::MultilingualE5Small);
        let assets =
            read_local_model_for_profile(&root.0, EmbeddingProfile::MultilingualE5Small).unwrap();
        assert_eq!(assets.onnx_file, b"invalid fixture: onnx/model.onnx");
        assert_eq!(assets.pooling, Some(Pooling::Mean));
        let err = load_error(FastEmbedEmbedder::load_from_model_dir_for_profile(
            &root.0,
            "multilingual-e5-small",
        ));
        assert!(err.contains("invalid onnx/model.onnx"), "{err}");
    }

    #[test]
    fn local_source_precedence_is_explicit_and_does_not_mutate_environment() {
        let select = |vars: &[(&str, &str)]| {
            local_source(|name| {
                vars.iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            })
        };
        match select(&[]).unwrap() {
            LocalSource::CacheDir(path) => assert_eq!(path, PathBuf::from(".fastembed_cache")),
            LocalSource::ModelDir(path) => assert!(
                path.ends_with("Library/Application Support/Evo/embedding"),
                "unexpected installed model path: {}",
                path.display()
            ),
        }
        assert_eq!(
            select(&[("FASTEMBED_CACHE_DIR", "fast")]).unwrap(),
            LocalSource::CacheDir("fast".into())
        );
        assert_eq!(
            select(&[("HF_HOME", "hf"), ("FASTEMBED_CACHE_DIR", "fast")]).unwrap(),
            LocalSource::CacheDir("hf".into())
        );
        assert_eq!(
            select(&[("EVO_EMBEDDING_MODEL_DIR", "exact"), ("HF_HOME", "hf")]).unwrap(),
            LocalSource::ModelDir("exact".into())
        );
        for variable in ["EVO_EMBEDDING_MODEL_DIR", "HF_HOME", "FASTEMBED_CACHE_DIR"] {
            let err = select(&[(variable, "")]).unwrap_err();
            assert!(err.contains(variable));
            assert!(err.contains("empty path"));
        }
        assert!(select(&[("EVO_EMBEDDING_MODEL_DIR", ""), ("HF_HOME", "hf")]).is_err());
    }

    #[test]
    fn empty_paths_and_missing_installations_fail_locally() {
        assert!(load_error(FastEmbedEmbedder::load_with_cache_dir(""))
            .contains("cache directory is empty"));
        assert!(load_error(FastEmbedEmbedder::load_from_model_dir(""))
            .contains("model directory is empty"));
        let root = FixtureDir::new();
        let err = load_error(FastEmbedEmbedder::load_with_cache_dir(&root.0));
        assert!(err.contains(
            &root
                .0
                .join(CACHE_MODEL_DIR)
                .join("refs/main")
                .display()
                .to_string()
        ));
        assert!(err.contains("no download or remote fallback"));
        // Loading must not provision folders or files in a missing cache.
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0);
        let err = load_error(FastEmbedEmbedder::load_from_model_dir(
            root.0.join("missing"),
        ));
        assert!(err.contains("missing/model.onnx"));
    }

    #[test]
    fn every_required_asset_must_exist_and_be_nonempty() {
        let root = FixtureDir::new();
        for file in MODEL_FILES {
            write_asset_markers(&root.0);
            let path = root.0.join(file);
            fs::remove_file(&path).unwrap();
            let err = load_error(FastEmbedEmbedder::load_from_model_dir(&root.0));
            assert!(err.contains(&path.display().to_string()), "{err}");
            assert!(err.contains("cannot read required asset"), "{err}");
            fs::write(&path, []).unwrap();
            let err = load_error(FastEmbedEmbedder::load_from_model_dir(&root.0));
            assert!(err.contains(&path.display().to_string()), "{err}");
            assert!(err.contains("is empty"), "{err}");
        }
    }

    #[test]
    fn directories_are_not_model_assets() {
        let root = FixtureDir::new();
        fs::create_dir(root.0.join("model.onnx")).unwrap();
        let err = load_error(FastEmbedEmbedder::load_from_model_dir(&root.0));
        assert!(err.contains("model.onnx' is not a regular file"), "{err}");
    }

    #[test]
    fn flat_directory_reads_exact_files_and_retains_model_parameters() {
        let root = FixtureDir::new();
        write_asset_markers(&root.0);
        let assets = read_local_model(&root.0).unwrap();
        assert_eq!(assets.onnx_file, b"invalid fixture: model.onnx");
        assert_eq!(
            assets.tokenizer_files.tokenizer_file,
            b"invalid fixture: tokenizer.json"
        );
        assert_eq!(
            assets.tokenizer_files.config_file,
            b"invalid fixture: config.json"
        );
        assert_eq!(
            assets.tokenizer_files.special_tokens_map_file,
            b"invalid fixture: special_tokens_map.json"
        );
        assert_eq!(
            assets.tokenizer_files.tokenizer_config_file,
            b"invalid fixture: tokenizer_config.json"
        );
        assert_eq!(
            assets.pooling,
            TextEmbedding::get_default_pooling_method(&EmbeddingModel::AllMiniLML6V2)
        );
        assert_eq!(assets.quantization, QuantizationMode::None);
        assert!(assets.external_initializers.is_empty());
        assert!(assets.output_key.is_none());
    }

    #[test]
    fn cache_resolution_follows_only_main_snapshot() {
        let root = FixtureDir::new();
        let selected = write_cache_ref(&root.0, "aabbcc\n");
        write_asset_markers(&selected);
        assert_eq!(cached_model_dir(&root.0).unwrap(), selected);
        assert!(read_local_model(&selected).is_ok());

        let other = root.0.join(CACHE_MODEL_DIR).join("snapshots/ddeeff");
        write_asset_markers(&other);
        fs::remove_file(selected.join("config.json")).unwrap();
        let err = load_error(FastEmbedEmbedder::load_with_cache_dir(&root.0));
        assert!(err.contains("snapshots/aabbcc/config.json"), "{err}");
        fs::remove_file(root.0.join(CACHE_MODEL_DIR).join("refs/main")).unwrap();
        assert!(cached_model_dir(&root.0)
            .unwrap_err()
            .contains("cannot read cache ref"));
    }

    #[test]
    fn malformed_cache_refs_fail_without_path_traversal() {
        let root = FixtureDir::new();
        for invalid in [
            "",
            "  \n",
            "..",
            "../aabbcc",
            "/aabbcc",
            "aabbcc/..",
            "ab\ncd",
            "other-model",
        ] {
            write_cache_ref(&root.0, invalid);
            let err = load_error(FastEmbedEmbedder::load_with_cache_dir(&root.0));
            assert!(err.contains("invalid cache ref"), "{invalid:?}: {err}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn cached_symlinks_to_local_blobs_are_supported() {
        let root = FixtureDir::new();
        let snapshot = write_cache_ref(&root.0, "aabbcc");
        fs::create_dir_all(&snapshot).unwrap();
        let blobs = root.0.join(CACHE_MODEL_DIR).join("blobs");
        write_asset_markers(&blobs);
        for file in MODEL_FILES {
            std::os::unix::fs::symlink(Path::new("../../blobs").join(file), snapshot.join(file))
                .unwrap();
        }
        assert!(read_local_model(&cached_model_dir(&root.0).unwrap()).is_ok());
        fs::remove_file(blobs.join("model.onnx")).unwrap();
        let err = load_error(FastEmbedEmbedder::load_with_cache_dir(&root.0));
        assert!(err.contains("snapshots/aabbcc/model.onnx"), "{err}");
    }

    #[test]
    fn corrupt_onnx_is_rejected_by_local_runtime_without_weights() {
        let root = FixtureDir::new();
        write_asset_markers(&root.0);
        let err = load_error(FastEmbedEmbedder::load_from_model_dir(&root.0));
        assert!(err.contains("invalid model.onnx"), "{err}");
        assert!(err.contains("ONNX runtime error"), "{err}");
        assert!(err.contains(&root.0.display().to_string()), "{err}");
    }

    #[test]
    fn doctor_checks_local_presence_without_claiming_hashes_or_inference() {
        let root = FixtureDir::new();
        write_asset_markers(&root.0);
        let report = FastEmbedEmbedder::doctor_from_model_dir(&root.0).unwrap();
        assert!(report.contains(&root.0.display().to_string()), "{report}");
        assert!(
            report.contains("five readable nonempty local assets"),
            "{report}"
        );
        assert!(
            report.contains("hashes and inference not checked"),
            "{report}"
        );
        // Intentionally corrupt bytes can pass presence, never actual loading.
        assert!(FastEmbedEmbedder::load_from_model_dir(&root.0).is_err());
        for name in MODEL_FILES {
            write_asset_markers(&root.0);
            fs::write(root.0.join(name), []).unwrap();
            let err = FastEmbedEmbedder::doctor_from_model_dir(&root.0).unwrap_err();
            assert!(err.contains(name), "{err}");
            assert!(err.contains("is empty"), "{err}");
        }
    }

    #[test]
    fn doctor_missing_directory_remains_missing() {
        let root = FixtureDir::new();
        let missing = root.0.join("missing");
        let err = FastEmbedEmbedder::doctor_from_model_dir(&missing).unwrap_err();
        assert!(err.contains("no download or remote fallback"), "{err}");
        assert!(err.contains("scripts/model/minilm.py fetch"), "{err}");
        assert!(!missing.exists());
        assert!(FastEmbedEmbedder::doctor_from_model_dir("").is_err());
    }

    #[test]
    #[ignore = "requires explicitly installed local model; never downloads"]
    fn real_model_offline_smoke() {
        let path = std::env::var_os("EVO_TEST_EMBEDDING_MODEL_DIR")
            .expect("set EVO_TEST_EMBEDDING_MODEL_DIR to verified local assets");
        let model = FastEmbedEmbedder::load_from_model_dir(PathBuf::from(path)).unwrap();
        let text = "A synthetic example about repairing a bicycle.";
        let first = model.embed(text).unwrap();
        let second = model.embed(text).unwrap();
        assert_eq!(first.len(), EMBEDDING_DIM);
        assert!(first.iter().all(|value| value.is_finite()));
        let norm = first.iter().map(|value| value * value).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001, "norm={norm}");
        assert!(first.iter().zip(&second).all(|(a, b)| (a - b).abs() < 1e-6));
        assert!(matches!(model.embed("  "), Err(EmbedError::Refused(_))));
    }

    #[test]
    #[ignore = "requires explicitly installed pinned local E5 assets; never downloads"]
    fn real_multilingual_e5_offline_smoke() {
        let path = std::env::var_os("EVO_TEST_E5_MODEL_DIR")
            .expect("set EVO_TEST_E5_MODEL_DIR to verified local E5 assets");
        let model = FastEmbedEmbedder::load_from_model_dir_for_profile(
            PathBuf::from(path),
            "multilingual-e5-small",
        )
        .unwrap();
        assert!(!model.allows_similarity_restore());
        assert_eq!(
            model.embedding_dimension(),
            MULTILINGUAL_E5_SMALL_EMBEDDING_DIM
        );
        let positive = model
            .embed_document("Instructions for repairing a bicycle tire.")
            .unwrap();
        let negative = model.embed_document("A recipe for lentil soup.").unwrap();
        for query in [
            "How do I repair a bicycle tire?",
            "كيف أصلح إطار الدراجة؟",
            "自転車のタイヤの修理方法",
            "साइकिल का टायर कैसे ठीक करें?",
        ] {
            let embedded = model.embed_query(query).unwrap();
            assert_eq!(embedded.len(), MULTILINGUAL_E5_SMALL_EMBEDDING_DIM);
            assert!(embedded.iter().all(|value| value.is_finite()));
            let relevant = evo_semantic::cosine_similarity(&embedded, &positive);
            let unrelated = evo_semantic::cosine_similarity(&embedded, &negative);
            assert!(
                relevant > unrelated,
                "synthetic cross-language retrieval did not prefer the relevant passage for {query:?}: {relevant} vs {unrelated}"
            );
        }
    }

    #[test]
    fn embedding_dimension_constant_matches_model() {
        let info = TextEmbedding::get_model_info(&EmbeddingModel::AllMiniLML6V2).unwrap();
        assert_eq!(EMBEDDING_DIM, info.dim);
        assert_eq!(info.model_code, "Qdrant/all-MiniLM-L6-v2-onnx");
        assert_eq!(info.model_file, MODEL_FILES[0]);
        assert!(info.additional_files.is_empty());
    }
}

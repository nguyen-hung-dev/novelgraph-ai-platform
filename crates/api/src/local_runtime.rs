use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use novelgraph_core::{
    AppConfig, LocalLlmDownloadState, LocalLlmModelSelection, LocalLlmPreset,
    LocalLlmRuntimeSnapshot,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs as tokio_fs,
    io::AsyncWriteExt,
    process::{Child, Command},
    sync::Mutex,
    task,
};

#[derive(Debug, Error)]
pub enum LocalRuntimeError {
    #[error("no model file was selected")]
    SelectionCancelled,
    #[error("unknown model preset: {0}")]
    UnknownPreset(String),
    #[error("another model download is already running")]
    DownloadAlreadyRunning,
    #[error("selected model file does not exist: {0}")]
    MissingModel(String),
    #[error("managed model path must stay inside the repo models directory")]
    ManagedModelOutsideRepo,
    #[error("invalid LLAMA_CPP_BASE_URL: {0}")]
    InvalidBaseUrl(String),
    #[error("failed to start llama-server: {0}")]
    StartFailed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct LocalLlmRuntimeManager {
    base_url: String,
    default_model_alias: String,
    server_binary: String,
    models_dir: PathBuf,
    state_file: PathBuf,
    http: reqwest::Client,
    inner: Arc<Mutex<LocalLlmRuntimeInner>>,
}

#[derive(Debug)]
struct LocalLlmRuntimeInner {
    selected_model: Option<PersistedModelSelection>,
    active_download: Option<LocalLlmDownloadState>,
    last_error: Option<String>,
    server_running: bool,
    server_pid: Option<u32>,
    child: Option<Child>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedRuntimeState {
    selected_model: Option<PersistedModelSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedModelSelection {
    source_kind: String,
    display_name: String,
    path: String,
    preset_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct LocalPresetSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    filename: &'static str,
    size_label: &'static str,
    source_url: &'static str,
}

const PRESETS: &[LocalPresetSpec] = &[
    LocalPresetSpec {
        id: "tinyllama-1.1b-q4-k-m",
        name: "TinyLlama 1.1B Chat Q4_K_M",
        description: "Nhỏ nhất và dễ khởi động nhất. Hợp để test pipeline local trên CPU.",
        filename: "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
        size_label: "~0.67 GB",
        source_url: "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf?download=true",
    },
    LocalPresetSpec {
        id: "qwen2.5-1.5b-q4-k-m",
        name: "Qwen2.5 1.5B Instruct Q4_K_M",
        description: "Cân bằng hơn TinyLlama cho tác vụ đọc, tóm tắt và prompt có cấu trúc.",
        filename: "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
        size_label: "~1.09 GB",
        source_url: "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf?download=true",
    },
    LocalPresetSpec {
        id: "smollm2-1.7b-q4-k-m",
        name: "SmolLM2 1.7B Instruct Q4_K_M",
        description: "Lựa chọn local-first nhỏ gọn hơn cho instruction và phân tích có cấu trúc.",
        filename: "smollm2-1.7b-instruct-q4_k_m.gguf",
        size_label: "~1.06 GB",
        source_url: "https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF/resolve/main/smollm2-1.7b-instruct-q4_k_m.gguf?download=true",
    },
];

impl LocalLlmRuntimeManager {
    pub async fn new(config: &AppConfig) -> Result<Self, LocalRuntimeError> {
        let repo_root = resolve_repo_root();
        let data_dir = repo_root.join("data");
        let models_dir = repo_root.join("models");
        tokio_fs::create_dir_all(&data_dir).await?;
        tokio_fs::create_dir_all(&models_dir).await?;

        let state_file = data_dir.join("local-llm-runtime.json");
        let persisted = load_persisted_state(&state_file).await?;
        let manager = Self {
            base_url: config.llama_cpp_base_url.clone(),
            default_model_alias: config.llama_cpp_default_model.clone(),
            server_binary: config.llama_cpp_server_bin.clone(),
            models_dir,
            state_file,
            http: reqwest::Client::new(),
            inner: Arc::new(Mutex::new(LocalLlmRuntimeInner {
                selected_model: persisted.selected_model,
                active_download: None,
                last_error: None,
                server_running: false,
                server_pid: None,
                child: None,
            })),
        };

        if let Err(error) = manager.start_selected_model().await {
            manager.set_last_error(Some(error.to_string())).await;
        }

        Ok(manager)
    }

    pub async fn snapshot(&self) -> LocalLlmRuntimeSnapshot {
        let (selected_model, active_download, last_error, server_running, server_pid) = {
            let mut inner = self.inner.lock().await;
            refresh_process_state(&mut inner);
            (
                inner.selected_model.clone(),
                inner.active_download.clone(),
                inner.last_error.clone(),
                inner.server_running,
                inner.server_pid,
            )
        };

        let downloaded_models = self.discover_downloaded_models();
        let selected_model = selected_model
            .as_ref()
            .map(|selection| build_model_selection(selection, Path::new(&selection.path)));
        let presets = PRESETS
            .iter()
            .map(|preset| {
                let installed = downloaded_models
                    .iter()
                    .any(|model| model.preset_id.as_deref() == Some(preset.id) && model.exists);
                let active = selected_model
                    .as_ref()
                    .and_then(|model| model.preset_id.as_deref())
                    == Some(preset.id);

                LocalLlmPreset {
                    id: preset.id.to_string(),
                    name: preset.name.to_string(),
                    description: preset.description.to_string(),
                    filename: preset.filename.to_string(),
                    size_label: preset.size_label.to_string(),
                    source_url: preset.source_url.to_string(),
                    installed,
                    active,
                }
            })
            .collect();

        LocalLlmRuntimeSnapshot {
            base_url: self.base_url.clone(),
            default_model_alias: self.default_model_alias.clone(),
            server_binary: self.server_binary.clone(),
            models_dir: self.models_dir.to_string_lossy().into_owned(),
            server_running,
            server_pid,
            selected_model,
            downloaded_models,
            presets,
            active_download,
            last_error,
        }
    }

    pub async fn pick_existing_model(&self) -> Result<LocalLlmRuntimeSnapshot, LocalRuntimeError> {
        let selected = task::spawn_blocking(|| {
            rfd::FileDialog::new()
                .add_filter("GGUF model", &["gguf"])
                .pick_file()
        })
        .await
        .map_err(|error| LocalRuntimeError::StartFailed(error.to_string()))?;

        let path = selected.ok_or(LocalRuntimeError::SelectionCancelled)?;
        if !path.exists() {
            return Err(LocalRuntimeError::MissingModel(
                path.to_string_lossy().into_owned(),
            ));
        }

        let selection = PersistedModelSelection {
            source_kind: "external".to_string(),
            display_name: file_display_name(&path),
            path: path.to_string_lossy().into_owned(),
            preset_id: None,
        };
        self.activate_selection(selection).await?;

        Ok(self.snapshot().await)
    }

    pub async fn start_selected_model(&self) -> Result<LocalLlmRuntimeSnapshot, LocalRuntimeError> {
        let selection = {
            let inner = self.inner.lock().await;
            inner.selected_model.clone()
        };

        if let Some(selection) = selection {
            self.activate_selection(selection).await?;
        }

        Ok(self.snapshot().await)
    }

    pub async fn stop_server(&self) -> Result<LocalLlmRuntimeSnapshot, LocalRuntimeError> {
        self.stop_server_process().await?;
        Ok(self.snapshot().await)
    }

    pub async fn activate_managed_model(
        &self,
        path: &str,
    ) -> Result<LocalLlmRuntimeSnapshot, LocalRuntimeError> {
        let target_path = PathBuf::from(path);
        let canonical_target = target_path.canonicalize()?;
        let canonical_models_dir = self.models_dir.canonicalize()?;
        if !canonical_target.starts_with(&canonical_models_dir) {
            return Err(LocalRuntimeError::ManagedModelOutsideRepo);
        }

        let preset_id = PRESETS
            .iter()
            .find(|preset| {
                preset
                    .filename
                    .eq_ignore_ascii_case(&file_name_string(&canonical_target))
            })
            .map(|preset| preset.id.to_string());
        let selection = PersistedModelSelection {
            source_kind: "managed".to_string(),
            display_name: file_display_name(&canonical_target),
            path: canonical_target.to_string_lossy().into_owned(),
            preset_id,
        };
        self.activate_selection(selection).await?;

        Ok(self.snapshot().await)
    }

    pub async fn download_or_activate_preset(
        &self,
        preset_id: &str,
    ) -> Result<LocalLlmRuntimeSnapshot, LocalRuntimeError> {
        let preset = preset_by_id(preset_id)
            .ok_or_else(|| LocalRuntimeError::UnknownPreset(preset_id.to_string()))?;
        let target_path = self.models_dir.join(preset.filename);

        if target_path.exists() {
            let selection = PersistedModelSelection {
                source_kind: "managed".to_string(),
                display_name: file_display_name(&target_path),
                path: target_path.to_string_lossy().into_owned(),
                preset_id: Some(preset.id.to_string()),
            };
            self.activate_selection(selection).await?;
            return Ok(self.snapshot().await);
        }

        {
            let mut inner = self.inner.lock().await;
            refresh_process_state(&mut inner);
            let busy = inner.active_download.as_ref().is_some_and(|download| {
                download.status == "queued" || download.status == "downloading"
            });
            if busy {
                return Err(LocalRuntimeError::DownloadAlreadyRunning);
            }

            inner.active_download = Some(LocalLlmDownloadState {
                preset_id: preset.id.to_string(),
                preset_name: preset.name.to_string(),
                filename: preset.filename.to_string(),
                target_path: target_path.to_string_lossy().into_owned(),
                status: "queued".to_string(),
                bytes_downloaded: 0,
                total_bytes: None,
                auto_activate: true,
                error_message: None,
            });
            inner.last_error = None;
        }

        let manager = self.clone();
        let preset = *preset;
        tokio::spawn(async move {
            manager.run_download_task(preset).await;
        });

        Ok(self.snapshot().await)
    }

    async fn activate_selection(
        &self,
        selection: PersistedModelSelection,
    ) -> Result<(), LocalRuntimeError> {
        let model_path = PathBuf::from(&selection.path);
        if !model_path.exists() {
            self.set_last_error(Some(format!(
                "selected model file does not exist: {}",
                model_path.to_string_lossy()
            )))
            .await;
            return Err(LocalRuntimeError::MissingModel(
                model_path.to_string_lossy().into_owned(),
            ));
        }

        self.stop_server_process().await?;

        match self.spawn_server(&model_path).await {
            Ok(child) => {
                let pid = child.id();
                {
                    let mut inner = self.inner.lock().await;
                    inner.selected_model = Some(selection);
                    inner.server_running = true;
                    inner.server_pid = pid;
                    inner.last_error = None;
                    inner.child = Some(child);
                }
                self.persist_selected_model().await?;
                Ok(())
            }
            Err(error) => {
                {
                    let mut inner = self.inner.lock().await;
                    inner.selected_model = Some(selection);
                    inner.server_running = false;
                    inner.server_pid = None;
                    inner.child = None;
                    inner.last_error = Some(error.to_string());
                }
                self.persist_selected_model().await?;
                Err(error)
            }
        }
    }

    async fn stop_server_process(&self) -> Result<(), LocalRuntimeError> {
        let child = {
            let mut inner = self.inner.lock().await;
            inner.server_running = false;
            inner.server_pid = None;
            inner.child.take()
        };

        if let Some(mut child) = child {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        Ok(())
    }

    async fn spawn_server(&self, model_path: &Path) -> Result<Child, LocalRuntimeError> {
        let url = Url::parse(&self.base_url)
            .map_err(|error| LocalRuntimeError::InvalidBaseUrl(error.to_string()))?;
        let host = url
            .host_str()
            .ok_or_else(|| LocalRuntimeError::InvalidBaseUrl("missing host".to_string()))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| LocalRuntimeError::InvalidBaseUrl("missing port".to_string()))?;

        let mut command = Command::new(&self.server_binary);
        command
            .kill_on_drop(true)
            .arg("--host")
            .arg(host)
            .arg("--port")
            .arg(port.to_string())
            .arg("-m")
            .arg(model_path)
            .arg("--alias")
            .arg(&self.default_model_alias)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        command.spawn().map_err(|error| {
            LocalRuntimeError::StartFailed(format!(
                "`{}` with model `{}`: {}",
                self.server_binary,
                model_path.to_string_lossy(),
                error
            ))
        })
    }

    async fn run_download_task(&self, preset: LocalPresetSpec) {
        let target_path = self.models_dir.join(preset.filename);
        let temp_path = self.models_dir.join(format!("{}.part", preset.filename));
        let result = self
            .download_preset_file(preset, &target_path, &temp_path)
            .await;

        match result {
            Ok(()) => {
                let selection = PersistedModelSelection {
                    source_kind: "managed".to_string(),
                    display_name: file_display_name(&target_path),
                    path: target_path.to_string_lossy().into_owned(),
                    preset_id: Some(preset.id.to_string()),
                };

                let activation_result = self.activate_selection(selection).await;
                let error_message = activation_result.err().map(|error| error.to_string());
                let mut inner = self.inner.lock().await;
                let previous_download = inner.active_download.clone();
                inner.active_download = Some(LocalLlmDownloadState {
                    preset_id: preset.id.to_string(),
                    preset_name: preset.name.to_string(),
                    filename: preset.filename.to_string(),
                    target_path: target_path.to_string_lossy().into_owned(),
                    status: if error_message.is_some() {
                        "completed_with_error".to_string()
                    } else {
                        "completed".to_string()
                    },
                    bytes_downloaded: previous_download
                        .as_ref()
                        .map(|download| download.bytes_downloaded)
                        .unwrap_or(0),
                    total_bytes: previous_download
                        .as_ref()
                        .and_then(|download| download.total_bytes),
                    auto_activate: true,
                    error_message: error_message.clone(),
                });
                if let Some(error_message) = error_message {
                    inner.last_error = Some(error_message);
                }
            }
            Err(error) => {
                let _ = tokio_fs::remove_file(&temp_path).await;
                let mut inner = self.inner.lock().await;
                let previous_download = inner.active_download.clone();
                inner.last_error = Some(error.to_string());
                inner.active_download = Some(LocalLlmDownloadState {
                    preset_id: preset.id.to_string(),
                    preset_name: preset.name.to_string(),
                    filename: preset.filename.to_string(),
                    target_path: target_path.to_string_lossy().into_owned(),
                    status: "failed".to_string(),
                    bytes_downloaded: previous_download
                        .as_ref()
                        .map(|download| download.bytes_downloaded)
                        .unwrap_or(0),
                    total_bytes: previous_download
                        .as_ref()
                        .and_then(|download| download.total_bytes),
                    auto_activate: true,
                    error_message: Some(error.to_string()),
                });
            }
        }
    }

    async fn download_preset_file(
        &self,
        preset: LocalPresetSpec,
        target_path: &Path,
        temp_path: &Path,
    ) -> Result<(), LocalRuntimeError> {
        let mut response = self.http.get(preset.source_url).send().await?;
        response.error_for_status_ref()?;
        let total_bytes = response.content_length();

        self.update_download_state(preset, "downloading", 0, total_bytes, None)
            .await;

        let mut file = tokio_fs::File::create(temp_path).await?;
        let mut downloaded = 0_u64;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            self.update_download_state(preset, "downloading", downloaded, total_bytes, None)
                .await;
        }

        file.flush().await?;
        tokio_fs::rename(temp_path, target_path).await?;
        self.update_download_state(preset, "downloaded", downloaded, total_bytes, None)
            .await;

        Ok(())
    }

    async fn update_download_state(
        &self,
        preset: LocalPresetSpec,
        status: &str,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
        error_message: Option<String>,
    ) {
        let target_path = self.models_dir.join(preset.filename);
        let mut inner = self.inner.lock().await;
        inner.active_download = Some(LocalLlmDownloadState {
            preset_id: preset.id.to_string(),
            preset_name: preset.name.to_string(),
            filename: preset.filename.to_string(),
            target_path: target_path.to_string_lossy().into_owned(),
            status: status.to_string(),
            bytes_downloaded,
            total_bytes,
            auto_activate: true,
            error_message,
        });
    }

    async fn persist_selected_model(&self) -> Result<(), LocalRuntimeError> {
        let selected_model = {
            let inner = self.inner.lock().await;
            inner.selected_model.clone()
        };
        let payload = PersistedRuntimeState { selected_model };
        let bytes = serde_json::to_vec_pretty(&payload)?;
        tokio_fs::write(&self.state_file, bytes).await?;
        Ok(())
    }

    async fn set_last_error(&self, message: Option<String>) {
        let mut inner = self.inner.lock().await;
        inner.last_error = message;
    }

    fn discover_downloaded_models(&self) -> Vec<LocalLlmModelSelection> {
        let mut models = Vec::new();
        let entries = match fs::read_dir(&self.models_dir) {
            Ok(entries) => entries,
            Err(_) => return models,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_gguf_path(&path) {
                continue;
            }

            let preset_id = PRESETS
                .iter()
                .find(|preset| {
                    preset
                        .filename
                        .eq_ignore_ascii_case(&file_name_string(&path))
                })
                .map(|preset| preset.id.to_string());
            let selection = PersistedModelSelection {
                source_kind: "managed".to_string(),
                display_name: file_display_name(&path),
                path: path.to_string_lossy().into_owned(),
                preset_id,
            };
            models.push(build_model_selection(&selection, &path));
        }

        models.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        models
    }
}

fn refresh_process_state(inner: &mut LocalLlmRuntimeInner) {
    let Some(child) = inner.child.as_mut() else {
        inner.server_running = false;
        inner.server_pid = None;
        return;
    };

    match child.try_wait() {
        Ok(Some(status)) => {
            inner.server_running = false;
            inner.server_pid = None;
            inner.child = None;
            inner.last_error = Some(format!("llama-server exited with status {status}"));
        }
        Ok(None) => {
            inner.server_running = true;
            inner.server_pid = child.id();
        }
        Err(error) => {
            inner.server_running = false;
            inner.server_pid = None;
            inner.child = None;
            inner.last_error = Some(format!("failed to read llama-server state: {error}"));
        }
    }
}

fn resolve_repo_root() -> PathBuf {
    if let Ok(current_dir) = std::env::current_dir() {
        if current_dir.join("Cargo.toml").exists()
            && current_dir.join("apps").is_dir()
            && current_dir.join("crates").is_dir()
        {
            return current_dir;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

async fn load_persisted_state(
    state_file: &Path,
) -> Result<PersistedRuntimeState, LocalRuntimeError> {
    match tokio_fs::read(state_file).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedRuntimeState::default())
        }
        Err(error) => Err(LocalRuntimeError::Io(error)),
    }
}

fn preset_by_id(preset_id: &str) -> Option<&'static LocalPresetSpec> {
    PRESETS.iter().find(|preset| preset.id == preset_id)
}

fn build_model_selection(
    selection: &PersistedModelSelection,
    path: &Path,
) -> LocalLlmModelSelection {
    let size_bytes = fs::metadata(path).ok().map(|metadata| metadata.len());
    LocalLlmModelSelection {
        source_kind: selection.source_kind.clone(),
        display_name: selection.display_name.clone(),
        path: selection.path.clone(),
        preset_id: selection.preset_id.clone(),
        size_bytes,
        exists: path.exists(),
    }
}

fn file_display_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn file_name_string(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn is_gguf_path(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::PRESETS;

    #[test]
    fn preset_catalog_has_unique_ids_and_filenames() {
        let ids = PRESETS
            .iter()
            .map(|preset| preset.id)
            .collect::<HashSet<_>>();
        let filenames = PRESETS
            .iter()
            .map(|preset| preset.filename)
            .collect::<HashSet<_>>();

        assert_eq!(ids.len(), PRESETS.len());
        assert_eq!(filenames.len(), PRESETS.len());
    }
}

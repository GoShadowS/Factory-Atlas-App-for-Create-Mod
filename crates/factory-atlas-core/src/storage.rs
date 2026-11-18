use crate::model::{AppSettings, FactoryProject};
use crate::project_format::{ProjectParseResult, parse_and_migrate_project, serialize_project};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub is_portable: bool,
    pub data_dir: PathBuf,
    pub projects_dir: PathBuf,
    pub assets_local_dir: PathBuf,
    pub assets_overrides_dir: PathBuf,
    pub settings_path: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let portable_flag =
            exe_dir.join("portable.flag").is_file() || exe_dir.join("user-projects").is_dir();

        if portable_flag {
            let projects_dir = exe_dir.join("user-projects");
            let assets_local_dir = exe_dir.join("assets-local");
            let assets_overrides_dir = exe_dir.join("assets-overrides");
            return Self {
                is_portable: true,
                data_dir: exe_dir.clone(),
                projects_dir,
                assets_local_dir,
                assets_overrides_dir,
                settings_path: exe_dir.join("settings.json"),
            };
        }

        let proj_dirs = ProjectDirs::from("dev", "FactoryAtlas", "FactoryAtlas");
        let data_dir = proj_dirs
            .as_ref()
            .map(|d| d.data_local_dir().to_path_buf())
            .unwrap_or_else(|| exe_dir.clone());
        let config_dir = proj_dirs
            .as_ref()
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| data_dir.clone());

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let assets_local_dir = if cwd.join("assets-local").is_dir() {
            cwd.join("assets-local")
        } else if exe_dir.join("assets-local").is_dir() {
            exe_dir.join("assets-local")
        } else {
            data_dir.join("assets-local")
        };

        let assets_overrides_dir = if cwd.join("assets-overrides").is_dir() {
            cwd.join("assets-overrides")
        } else if exe_dir.join("assets-overrides").is_dir() {
            exe_dir.join("assets-overrides")
        } else {
            data_dir.join("assets-overrides")
        };

        Self {
            is_portable: false,
            projects_dir: data_dir.join("projects"),
            data_dir,
            assets_local_dir,
            assets_overrides_dir,
            settings_path: config_dir.join("settings.json"),
        }
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        fs::create_dir_all(&self.projects_dir)
            .map_err(|e| format!("Failed to create projects dir: {e}"))?;
        if let Some(p) = self.settings_path.parent() {
            fs::create_dir_all(p).map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        Ok(())
    }

    pub fn load_settings(&self) -> AppSettings {
        fs::read_to_string(&self.settings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        self.ensure_dirs()?;
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {e}"))?;
        atomic_write_string(&self.settings_path, &json)
    }
}

pub fn atomic_write_string(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dir for {}: {e}", path.display()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("Failed to commit {}: {e}", path.display()))?;
    Ok(())
}

pub fn load_project_file(path: &Path) -> Result<(FactoryProject, Option<u32>), String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read project {}: {e}", path.display()))?;
    match parse_and_migrate_project(&raw) {
        ProjectParseResult::Ok {
            project,
            migrated_from_version,
        } => Ok((*project, migrated_from_version)),
        ProjectParseResult::Err { error_en, .. } => Err(error_en),
    }
}

pub fn save_project_file(path: &Path, project: &FactoryProject) -> Result<(), String> {
    let json =
        serialize_project(project).map_err(|e| format!("Failed to serialize project: {e}"))?;
    atomic_write_string(path, &json)
}

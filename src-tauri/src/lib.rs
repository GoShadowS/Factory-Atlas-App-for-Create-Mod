use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContextInfo {
    pub runtime: String,
    pub mode: String, // "installed" | "portable"
    pub data_dir: String,
    pub projects_dir: String,
    pub overrides_dir: String,
    pub initial_open_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFilePayload {
    pub path: String,
    pub file_name: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskOverrideItem {
    pub rel_path: String,
    pub data_url: String,
}

struct AppRuntimeState {
    mode: String,
    data_dir: PathBuf,
    projects_dir: PathBuf,
    overrides_dir: PathBuf,
    bundled_overrides_dir: Option<PathBuf>,
    initial_open_file: Mutex<Option<String>>,
}

fn detect_portable_mode(exe_dir: Option<&Path>) -> bool {
    if std::env::args().any(|a| a == "--portable") {
        return true;
    }
    if let Some(dir) = exe_dir {
        if dir.join("portable.flag").exists() || dir.join(".portable").exists() {
            return true;
        }
        if dir.join("data").is_dir() && !dir.join("uninstall.exe").exists() {
            return true;
        }
    }
    false
}

fn is_safe_override_rel(rel: &str) -> bool {
    if rel.is_empty() || rel.contains('\0') || rel.contains('\\') || rel.starts_with('/') {
        return false;
    }
    if !rel.ends_with(".png") {
        return false;
    }
    !rel.split('/').any(|part| part == "..")
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if clean.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("Invalid base64 character".to_string()),
        }
    };
    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    for chunk in clean.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            let c = val(chunk[2])?;
            out.push((b << 4) | (c >> 2));
            if chunk[3] != b'=' {
                let d = val(chunk[3])?;
                out.push((c << 6) | d);
            }
        }
    }
    Ok(out)
}

fn collect_png_overrides(base: &Path, cur: &Path, out: &mut Vec<DiskOverrideItem>) {
    let Ok(entries) = fs::read_dir(cur) else {
        return;
    };
    for ent in entries.flatten() {
        let path = ent.path();
        if path.is_dir() {
            collect_png_overrides(base, &path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("png") {
            if let (Ok(rel), Ok(bytes)) = (path.strip_prefix(base), fs::read(&path)) {
                if bytes.len() >= 8 && bytes[0] == 0x89 && bytes[1] == 0x50 {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    out.push(DiskOverrideItem {
                        rel_path: rel_str,
                        data_url: format!("data:image/png;base64,{}", base64_encode(&bytes)),
                    });
                }
            }
        }
    }
}

#[tauri::command]
fn get_runtime_context(state: State<'_, AppRuntimeState>) -> RuntimeContextInfo {
    let initial = state
        .initial_open_file
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    RuntimeContextInfo {
        runtime: "tauri-desktop".to_string(),
        mode: state.mode.clone(),
        data_dir: state.data_dir.to_string_lossy().to_string(),
        projects_dir: state.projects_dir.to_string_lossy().to_string(),
        overrides_dir: state.overrides_dir.to_string_lossy().to_string(),
        initial_open_file_path: initial,
    }
}

#[tauri::command]
fn desktop_load_autosave(state: State<'_, AppRuntimeState>) -> Result<Option<String>, String> {
    let path = state.data_dir.join("autosave.json");
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("Failed reading autosave: {e}"))
}

#[tauri::command]
fn desktop_save_autosave(state: State<'_, AppRuntimeState>, payload: String) -> Result<(), String> {
    fs::create_dir_all(&state.data_dir).map_err(|e| e.to_string())?;
    let tmp = state.data_dir.join("autosave.json.tmp");
    let target = state.data_dir.join("autosave.json");
    fs::write(&tmp, payload.as_bytes()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())
}

#[tauri::command]
fn desktop_save_project_with_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppRuntimeState>,
    default_name: String,
    json_text: String,
) -> Result<Option<String>, String> {
    fs::create_dir_all(&state.projects_dir).map_err(|e| e.to_string())?;
    let chosen = app
        .dialog()
        .file()
        .set_directory(&state.projects_dir)
        .set_file_name(&default_name)
        .add_filter("Factory Atlas Project", &["factoryatlas", "json"])
        .blocking_save_file();

    let Some(file_path) = chosen else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Unsupported file URI: {e}"))?;
    fs::write(&path, json_text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn desktop_open_project_with_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppRuntimeState>,
) -> Result<Option<OpenedFilePayload>, String> {
    fs::create_dir_all(&state.projects_dir).map_err(|e| e.to_string())?;
    let chosen = app
        .dialog()
        .file()
        .set_directory(&state.projects_dir)
        .add_filter("Factory Atlas Project", &["factoryatlas", "json"])
        .blocking_pick_file();

    let Some(file_path) = chosen else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Unsupported file URI: {e}"))?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project.factoryatlas")
        .to_string();
    Ok(Some(OpenedFilePayload {
        path: path.to_string_lossy().to_string(),
        file_name,
        text,
    }))
}

#[tauri::command]
fn desktop_read_file_text(path: String) -> Result<OpenedFilePayload, String> {
    let p = PathBuf::from(&path);
    let text = fs::read_to_string(&p).map_err(|e| format!("Cannot read project file: {e}"))?;
    let file_name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project.factoryatlas")
        .to_string();
    Ok(OpenedFilePayload {
        path,
        file_name,
        text,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportDialogRequest {
    default_name: String,
    filter_name: String,
    extensions: Vec<String>,
    base64_bytes: String,
}

#[tauri::command]
fn desktop_save_export_with_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppRuntimeState>,
    req: ExportDialogRequest,
) -> Result<Option<String>, String> {
    let bytes = base64_decode(&req.base64_bytes)?;
    let ext_refs: Vec<&str> = req.extensions.iter().map(|s| s.as_str()).collect();
    let chosen = app
        .dialog()
        .file()
        .set_directory(&state.projects_dir)
        .set_file_name(&req.default_name)
        .add_filter(&req.filter_name, &ext_refs)
        .blocking_save_file();

    let Some(file_path) = chosen else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|e| format!("Unsupported file URI: {e}"))?;
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn desktop_list_overrides(state: State<'_, AppRuntimeState>) -> Vec<DiskOverrideItem> {
    let mut items = Vec::new();
    if let Some(ref bundled) = state.bundled_overrides_dir {
        collect_png_overrides(bundled, bundled, &mut items);
    }
    collect_png_overrides(&state.overrides_dir, &state.overrides_dir, &mut items);
    items
}

#[tauri::command]
fn desktop_save_override_png(
    state: State<'_, AppRuntimeState>,
    rel_path: String,
    data_url: String,
) -> Result<(), String> {
    if !is_safe_override_rel(&rel_path) {
        return Err("Invalid PNG override path".to_string());
    }
    let Some(b64) = data_url.strip_prefix("data:image/png;base64,") else {
        return Err("Expected PNG data URL".to_string());
    };
    let bytes = base64_decode(b64)?;
    if bytes.len() > 4 * 1024 * 1024 || bytes.len() < 8 || bytes[0] != 0x89 || bytes[1] != 0x50 {
        return Err("Invalid PNG payload or exceeds 4 MB".to_string());
    }
    let target = state.overrides_dir.join(&rel_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, &bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn desktop_remove_override_png(
    state: State<'_, AppRuntimeState>,
    rel_path: String,
) -> Result<(), String> {
    if !is_safe_override_rel(&rel_path) {
        return Err("Invalid PNG override path".to_string());
    }
    let target = state.overrides_dir.join(&rel_path);
    if target.exists() {
        fs::remove_file(&target).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()));
            let is_portable = detect_portable_mode(exe_dir.as_deref());

            let (mode, data_dir, webview_dir) = if is_portable {
                let root = exe_dir.clone().unwrap_or_else(|| PathBuf::from("."));
                let data = root.join("data");
                let wv = data.join("webview");
                ("portable".to_string(), data, wv)
            } else {
                let data = app.path().app_data_dir()?;
                let local = app.path().app_local_data_dir()?;
                let wv = local.join("webview");
                ("installed".to_string(), data, wv)
            };

            let projects_dir = data_dir.join("projects");
            let overrides_dir = data_dir.join("assets-overrides");
            let _ = fs::create_dir_all(&projects_dir);
            let _ = fs::create_dir_all(&overrides_dir);
            let _ = fs::create_dir_all(&webview_dir);

            let bundled_overrides_dir = exe_dir
                .as_ref()
                .map(|d| d.join("assets-overrides"))
                .filter(|p| p.is_dir());

            let initial_open = std::env::args()
                .skip(1)
                .find(|a| !a.starts_with('-') && (a.ends_with(".factoryatlas") || a.ends_with(".json")) && Path::new(a).is_file());

            app.manage(AppRuntimeState {
                mode: mode.clone(),
                data_dir,
                projects_dir,
                overrides_dir,
                bundled_overrides_dir,
                initial_open_file: Mutex::new(initial_open),
            });

            if let Some(win_cfg) = app.config().app.windows.first() {
                let mut builder = WebviewWindowBuilder::from_config(app.handle(), win_cfg)?
                    .data_directory(webview_dir);
                if mode == "portable" {
                    builder = builder.title("Factory Atlas — Планировщик Create (Портативный режим)");
                }
                builder.build()?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_runtime_context,
            desktop_load_autosave,
            desktop_save_autosave,
            desktop_save_project_with_dialog,
            desktop_open_project_with_dialog,
            desktop_read_file_text,
            desktop_save_export_with_dialog,
            desktop_list_overrides,
            desktop_save_override_png,
            desktop_remove_override_png,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Factory Atlas Tauri application");
}

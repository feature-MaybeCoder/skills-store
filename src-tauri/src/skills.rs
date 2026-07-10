use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Root of the app's master store, relative to the user's home directory.
const STORE_DIR: &str = ".skills-store";
/// Canonical target convention where *enabled* skills live.
const AGENTS_REL: &str = ".agents/skills";
/// Legacy convention, imported into the store on sync but never used as an enable target.
const CODEX_REL: &str = ".codex/skills";

fn e2s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn home() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "cannot resolve home directory".to_string())
}

fn store_root() -> Result<PathBuf, String> {
    Ok(home()?.join(STORE_DIR))
}

fn store_global_dir() -> Result<PathBuf, String> {
    Ok(store_root()?.join("global"))
}

fn store_projects_dir() -> Result<PathBuf, String> {
    Ok(store_root()?.join("project"))
}

fn global_target() -> Result<PathBuf, String> {
    Ok(home()?.join(AGENTS_REL))
}

fn global_legacy() -> Result<PathBuf, String> {
    Ok(home()?.join(CODEX_REL))
}

fn project_target(project_path: &Path) -> PathBuf {
    project_path.join(AGENTS_REL)
}

fn project_legacy(project_path: &Path) -> PathBuf {
    project_path.join(CODEX_REL)
}

fn codex_config_path() -> Result<PathBuf, String> {
    Ok(home()?.join(".codex/config.toml"))
}

/// A directory is treated as a "project" when it holds any skills.
fn project_has_skills(root: &Path) -> bool {
    !list_skill_dirs(&project_target(root)).is_empty()
        || !list_skill_dirs(&project_legacy(root)).is_empty()
}

/// Absolute project paths registered in Codex's `~/.codex/config.toml`
/// (the `[projects."<path>"]` tables).
fn codex_project_paths() -> Vec<String> {
    let Ok(path) = codex_config_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return Vec::new();
    };
    value
        .get("projects")
        .and_then(|v| v.as_table())
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    /// Folder name; the stable identity used for all paths.
    dir_name: String,
    /// `name` from frontmatter, falls back to `dir_name`.
    name: String,
    description: String,
    /// True when the skill currently exists in its target location.
    enabled: bool,
    /// True when SKILL.md frontmatter parsed successfully.
    valid: bool,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// Folder name under `~/.skills-store/project/`.
    name: String,
    /// Absolute path to the real project, from its `config.yaml`.
    path: String,
    /// True when `path` exists on disk.
    exists: bool,
    skills: Vec<SkillMeta>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreState {
    root: String,
    global: Vec<SkillMeta>,
    projects: Vec<ProjectInfo>,
}

#[derive(Deserialize, Default)]
struct FrontMatter {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ProjectConfig {
    path: String,
}

/// Identifies a single skill for mutating commands.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillRef {
    /// "global" | "project"
    scope: String,
    /// Store project folder name; required when `scope == "project"`.
    project: Option<String>,
    dir_name: String,
}

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(e2s)
}

/// Recursively copy a directory, resolving symlinks to their real content so
/// symlinked skill folders are materialized in the destination.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(e2s)?;
    for entry in fs::read_dir(src).map_err(e2s)? {
        let entry = entry.map_err(e2s)?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type().map_err(e2s)?;
        if ty.is_symlink() {
            let real = fs::canonicalize(&from).map_err(e2s)?;
            if real.is_dir() {
                copy_dir_all(&real, &to)?;
            } else {
                fs::copy(&real, &to).map_err(e2s)?;
            }
        } else if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(e2s)?;
        }
    }
    Ok(())
}

/// Replace `dst` with a fresh copy of `src`.
fn replace_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(e2s)?;
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    copy_dir_all(src, dst)
}

/// Immediate subdirectories that look like a skill (contain SKILL.md).
fn list_skill_dirs(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() && path.join("SKILL.md").is_file() {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

fn parse_frontmatter(skill_dir: &Path) -> (Option<FrontMatter>, Option<String>) {
    let md = skill_dir.join("SKILL.md");
    let content = match fs::read_to_string(&md) {
        Ok(c) => c,
        Err(e) => return (None, Some(format!("SKILL.md not readable: {e}"))),
    };
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    if !trimmed.starts_with("---") {
        return (None, Some("missing YAML frontmatter".into()));
    }
    let after = &trimmed[3..];
    let Some(end) = after.find("\n---") else {
        return (None, Some("unterminated YAML frontmatter".into()));
    };
    match serde_yaml::from_str::<FrontMatter>(&after[..end]) {
        Ok(fm) => (Some(fm), None),
        Err(e) => (None, Some(format!("invalid YAML: {e}"))),
    }
}

fn build_meta(store_skill: &Path, dir_name: &str, target_root: &Path) -> SkillMeta {
    let (fm, error) = parse_frontmatter(store_skill);
    let (name, description, valid) = match fm {
        Some(f) => (
            f.name.unwrap_or_else(|| dir_name.to_string()),
            f.description.unwrap_or_default(),
            true,
        ),
        None => (dir_name.to_string(), String::new(), false),
    };
    let enabled = target_root.join(dir_name).join("SKILL.md").is_file();
    SkillMeta {
        dir_name: dir_name.to_string(),
        name,
        description,
        enabled,
        valid,
        error,
    }
}

/// Import every skill found in `source_root` into `store_dir`.
/// When `overwrite` is false, skills already present in the store are left untouched.
fn import_from(source_root: &Path, store_dir: &Path, overwrite: bool) -> Result<(), String> {
    if !source_root.exists() {
        return Ok(());
    }
    ensure_dir(store_dir)?;
    for dir_name in list_skill_dirs(source_root) {
        let dst = store_dir.join(&dir_name);
        if dst.exists() && !overwrite {
            continue;
        }
        replace_dir(&source_root.join(&dir_name), &dst)?;
    }
    Ok(())
}

fn project_path_from_config(project_store: &Path) -> String {
    fs::read_to_string(project_store.join("config.yaml"))
        .ok()
        .and_then(|c| serde_yaml::from_str::<ProjectConfig>(&c).ok())
        .map(|c| c.path)
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// State reading
// ---------------------------------------------------------------------------

fn read_state() -> Result<StoreState, String> {
    let root = store_root()?;
    ensure_dir(&store_global_dir()?)?;
    ensure_dir(&store_projects_dir()?)?;

    let g_store = store_global_dir()?;
    let g_target = global_target()?;
    let global = list_skill_dirs(&g_store)
        .iter()
        .map(|n| build_meta(&g_store.join(n), n, &g_target))
        .collect();

    let mut projects = Vec::new();
    if let Ok(rd) = fs::read_dir(store_projects_dir()?) {
        for entry in rd.flatten() {
            let pdir = entry.path();
            if !pdir.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let path = project_path_from_config(&pdir);
            let target_root = if path.is_empty() {
                PathBuf::new()
            } else {
                project_target(Path::new(&path))
            };
            let skills = list_skill_dirs(&pdir)
                .iter()
                .map(|n| build_meta(&pdir.join(n), n, &target_root))
                .collect();
            let exists = !path.is_empty() && Path::new(&path).is_dir();
            projects.push(ProjectInfo {
                name,
                path,
                exists,
                skills,
            });
        }
    }
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(StoreState {
        root: root.to_string_lossy().into_owned(),
        global,
        projects,
    })
}

// ---------------------------------------------------------------------------
// Mutations
// ---------------------------------------------------------------------------

fn resolve_store_skill(skill: &SkillRef) -> Result<PathBuf, String> {
    match skill.scope.as_str() {
        "global" => Ok(store_global_dir()?.join(&skill.dir_name)),
        "project" => {
            let project = skill
                .project
                .as_ref()
                .ok_or_else(|| "project name required for project scope".to_string())?;
            Ok(store_projects_dir()?.join(project).join(&skill.dir_name))
        }
        other => Err(format!("unknown scope: {other}")),
    }
}

fn resolve_target_skill(skill: &SkillRef) -> Result<PathBuf, String> {
    match skill.scope.as_str() {
        "global" => Ok(global_target()?.join(&skill.dir_name)),
        "project" => {
            let project = skill
                .project
                .as_ref()
                .ok_or_else(|| "project name required for project scope".to_string())?;
            let path = project_path_from_config(&store_projects_dir()?.join(project));
            if path.is_empty() {
                return Err(format!("project '{project}' has no configured path"));
            }
            Ok(project_target(Path::new(&path)).join(&skill.dir_name))
        }
        other => Err(format!("unknown scope: {other}")),
    }
}

fn set_enabled(skill: &SkillRef, enabled: bool) -> Result<(), String> {
    let store_skill = resolve_store_skill(skill)?;
    let target_skill = resolve_target_skill(skill)?;
    if enabled {
        if !store_skill.join("SKILL.md").is_file() {
            return Err(format!(
                "skill '{}' is not present in the store",
                skill.dir_name
            ));
        }
        replace_dir(&store_skill, &target_skill)?;
    } else if target_skill.exists() {
        fs::remove_dir_all(&target_skill).map_err(e2s)?;
    }
    Ok(())
}

fn delete_skill_impl(skill: &SkillRef) -> Result<(), String> {
    // Remove from target first (best-effort), then from the store.
    if let Ok(target_skill) = resolve_target_skill(skill) {
        if target_skill.exists() {
            fs::remove_dir_all(&target_skill).map_err(e2s)?;
        }
    }
    let store_skill = resolve_store_skill(skill)?;
    if store_skill.exists() {
        fs::remove_dir_all(&store_skill).map_err(e2s)?;
    }
    Ok(())
}

/// Store folder name of an already-registered project with the given real path.
fn find_project_by_path(projects_dir: &Path, path: &str) -> Option<String> {
    let rd = fs::read_dir(projects_dir).ok()?;
    for entry in rd.flatten() {
        let pdir = entry.path();
        if pdir.is_dir() && project_path_from_config(&pdir) == path {
            return Some(entry.file_name().to_string_lossy().to_string());
        }
    }
    None
}

/// Register a project in the store (creating `config.yaml`) if not already present.
/// Returns the store folder name.
fn register_project(projects_dir: &Path, path: &str) -> Result<String, String> {
    if let Some(existing) = find_project_by_path(projects_dir, path) {
        return Ok(existing);
    }
    let base = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = unique_project_name(projects_dir, &base, path);
    let pdir = projects_dir.join(&name);
    ensure_dir(&pdir)?;
    let yaml = serde_yaml::to_string(&ProjectConfig {
        path: path.to_string(),
    })
    .map_err(e2s)?;
    fs::write(pdir.join("config.yaml"), yaml).map_err(e2s)?;
    Ok(name)
}

/// Pick a store folder name for a project, avoiding collisions with unrelated projects.
fn unique_project_name(projects_dir: &Path, base: &str, path: &str) -> String {
    let base = if base.is_empty() { "project" } else { base };
    let mut candidate = base.to_string();
    let mut i = 2;
    loop {
        let dir = projects_dir.join(&candidate);
        if !dir.exists() || project_path_from_config(&dir) == path {
            return candidate;
        }
        candidate = format!("{base}-{i}");
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_state() -> Result<StoreState, String> {
    read_state()
}

#[tauri::command]
pub fn sync_all() -> Result<StoreState, String> {
    // Global: target is the source of truth, legacy is imported once.
    import_from(&global_target()?, &store_global_dir()?, true)?;
    import_from(&global_legacy()?, &store_global_dir()?, false)?;

    let projects_dir = store_projects_dir()?;
    ensure_dir(&projects_dir)?;

    // Auto-discover projects: any Codex-registered path that holds skills.
    for path in codex_project_paths() {
        let root = Path::new(&path);
        if root.is_dir() && project_has_skills(root) {
            register_project(&projects_dir, &path)?;
        }
    }

    // Projects: sync each registered project from its real location.
    if let Ok(rd) = fs::read_dir(&projects_dir) {
        for entry in rd.flatten() {
            let pdir = entry.path();
            if !pdir.is_dir() {
                continue;
            }
            let path = project_path_from_config(&pdir);
            if path.is_empty() {
                continue;
            }
            let root = Path::new(&path);
            import_from(&project_target(root), &pdir, true)?;
            import_from(&project_legacy(root), &pdir, false)?;
        }
    }

    read_state()
}

#[tauri::command]
pub fn add_project(path: String) -> Result<StoreState, String> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let projects_dir = store_projects_dir()?;
    ensure_dir(&projects_dir)?;
    let name = register_project(&projects_dir, &path)?;
    let pdir = projects_dir.join(&name);

    import_from(&project_target(root), &pdir, true)?;
    import_from(&project_legacy(root), &pdir, false)?;

    read_state()
}

/// Copy a global skill into a project's store and enable it in the project target.
#[tauri::command]
pub fn add_skill_to_project(project: String, dir_name: String) -> Result<StoreState, String> {
    let src = store_global_dir()?.join(&dir_name);
    if !src.join("SKILL.md").is_file() {
        return Err(format!("global skill '{dir_name}' not found"));
    }
    let pdir = store_projects_dir()?.join(&project);
    if !pdir.is_dir() {
        return Err(format!("project '{project}' not found"));
    }
    replace_dir(&src, &pdir.join(&dir_name))?;
    set_enabled(
        &SkillRef {
            scope: "project".to_string(),
            project: Some(project),
            dir_name,
        },
        true,
    )?;
    read_state()
}

#[tauri::command]
pub fn remove_project(name: String) -> Result<StoreState, String> {
    let pdir = store_projects_dir()?.join(&name);
    if pdir.exists() {
        fs::remove_dir_all(&pdir).map_err(e2s)?;
    }
    read_state()
}

/// Import an external skill folder (or a folder of skills) into the global store.
#[tauri::command]
pub fn import_global_skill(source: String) -> Result<StoreState, String> {
    let src = Path::new(&source);
    if !src.is_dir() {
        return Err(format!("not a directory: {source}"));
    }
    let store = store_global_dir()?;
    ensure_dir(&store)?;
    if src.join("SKILL.md").is_file() {
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| "invalid skill folder name".to_string())?;
        replace_dir(src, &store.join(name))?;
    } else {
        import_from(src, &store, true)?;
    }
    read_state()
}

#[tauri::command]
pub fn set_skills_enabled(skills: Vec<SkillRef>, enabled: bool) -> Result<StoreState, String> {
    for skill in &skills {
        set_enabled(skill, enabled)?;
    }
    read_state()
}

#[tauri::command]
pub fn delete_skills(skills: Vec<SkillRef>) -> Result<StoreState, String> {
    for skill in &skills {
        delete_skill_impl(skill)?;
    }
    read_state()
}

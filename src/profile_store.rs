//! Install and load compiled API profiles (YAML source → `.pb` runtime).
//!
//! Runtime callers should use [`load_installed_profile`] (native filesystem) or
//! [`load_from_proto_bytes`] (embedded bytes). YAML is only parsed during install/compile.

use std::path::{Path, PathBuf};

use crate::error::{Result, YcallrError};
use crate::models::ApiDefinition;
use crate::yaml_parser;

/// Default profile directory: `~/.config/ycallr/apis`.
pub fn apis_dir() -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dirs::home_dir()
            .map(|home| home.join(".config").join("ycallr").join("apis"))
            .unwrap_or_else(|| PathBuf::from(".config/ycallr/apis"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from(".config/ycallr/apis")
    }
}

pub fn yaml_profile_path(name: &str) -> PathBuf {
    apis_dir().join(format!("{}.yaml", name))
}

pub fn compiled_profile_path(name: &str) -> PathBuf {
    apis_dir().join(format!("{}.pb", name))
}

/// Parse YAML, validate for client use, and return protobuf bytes (install step).
pub fn compile_yaml_str(yaml: &str) -> Result<Vec<u8>> {
    let api = yaml_parser::parse_yaml_for_client(yaml)?;
    api.to_proto_bytes()
}

/// Read YAML from disk, validate, and return protobuf bytes.
pub fn compile_yaml_file(path: &Path) -> Result<Vec<u8>> {
    let api = yaml_parser::parse_yaml_file_for_client(path)?;
    api.to_proto_bytes()
}

/// Decode an installed protobuf profile for client execution.
pub fn load_from_proto_bytes(bytes: &[u8]) -> Result<ApiDefinition> {
    ApiDefinition::from_proto_bytes_for_client(bytes)
}

/// Human-readable hint when a profile is missing on disk.
pub fn not_installed_message(name: &str) -> String {
    let yaml_path = yaml_profile_path(name);
    let mut msg = format!("API profile '{}' is not installed.", name);
    if yaml_path.is_file() {
        msg.push_str(&format!(
            "\nInstall with: ycallr install {}",
            yaml_path.display()
        ));
    } else {
        msg.push_str("\nInstall with: ycallr install <path/to/profile.yaml>");
    }
    msg
}

/// Expand `~` and resolve `.yaml` / `.yml` when the path has no extension.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_yaml_source_path(path: &Path) -> Result<PathBuf> {
    let expanded = expand_home_path(path);
    if expanded.is_file() {
        return Ok(expanded);
    }

    let has_yaml_ext = expanded
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false);
    if has_yaml_ext {
        return Err(YcallrError::ProfileInstall(format!(
            "YAML file not found: {}",
            expanded.display()
        )));
    }

    for ext in ["yaml", "yml"] {
        let candidate = expanded.with_extension(ext);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let with_yaml = expanded.with_extension("yaml");
    Err(YcallrError::ProfileInstall(format!(
        "YAML file not found: {} (also tried {} and {})",
        expanded.display(),
        with_yaml.display(),
        expanded.with_extension("yml").display()
    )))
}

#[cfg(not(target_arch = "wasm32"))]
fn expand_home_path(path: &Path) -> PathBuf {
    if let Some(s) = path.to_str() {
        if s == "~" {
            return dirs::home_dir().unwrap_or_else(|| path.to_path_buf());
        }
        if let Some(rest) = s.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        }
    }
    path.to_path_buf()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_apis_dir() -> Result<PathBuf> {
    let dir = apis_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Compile `~/.config/ycallr/apis/<name>.yaml` → `<name>.pb`.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_profile(name: &str) -> Result<PathBuf> {
    ensure_apis_dir()?;
    let yaml_path = yaml_profile_path(name);
    if !yaml_path.is_file() {
        return Err(YcallrError::ProfileInstall(format!(
            "YAML profile not found: {}\nPlace {} or pass a path to a .yaml file.",
            yaml_path.display(),
            yaml_path.display()
        )));
    }
    install_profile_yaml_path(name, &yaml_path)
}

/// Install from an arbitrary YAML file path (copies into apis dir when path differs).
#[cfg(not(target_arch = "wasm32"))]
pub fn install_profile_from_path(source: &Path) -> Result<(String, PathBuf)> {
    ensure_apis_dir()?;

    let source = resolve_yaml_source_path(source)?;

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if ext != "yaml" && ext != "yml" {
        return Err(YcallrError::ProfileInstall(format!(
            "Expected a .yaml or .yml file, got: {}",
            source.display()
        )));
    }

    let api = yaml_parser::parse_yaml_file_for_client(&source)?;
    let name = api.name.clone();
    let proto_bytes = api.to_proto_bytes()?;

    let dest_yaml = yaml_profile_path(&name);
    if source.canonicalize().ok() != dest_yaml.canonicalize().ok() {
        std::fs::copy(&source, &dest_yaml).map_err(|e| {
            YcallrError::ProfileInstall(format!(
                "Failed to copy YAML to {}: {}",
                dest_yaml.display(),
                e
            ))
        })?;
    }

    let pb_path = compiled_profile_path(&name);
    std::fs::write(&pb_path, &proto_bytes)?;
    Ok((name, pb_path))
}

#[cfg(not(target_arch = "wasm32"))]
fn install_profile_yaml_path(name: &str, yaml_path: &Path) -> Result<PathBuf> {
    let proto_bytes = compile_yaml_file(yaml_path)?;
    let pb_path = compiled_profile_path(name);
    std::fs::write(&pb_path, &proto_bytes)?;
    Ok(pb_path)
}

/// Load `~/.config/ycallr/apis/<name>.pb` (fails if not installed).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_installed_profile(name: &str) -> Result<ApiDefinition> {
    let pb_path = compiled_profile_path(name);
    if !pb_path.is_file() {
        return Err(YcallrError::ProfileNotInstalled {
            name: name.to_string(),
            message: not_installed_message(name),
        });
    }

    let bytes = std::fs::read(&pb_path)?;
    load_from_proto_bytes(&bytes).map_err(|e| {
        YcallrError::ProfileInstall(format!(
            "Failed to decode installed profile '{}': {}",
            name, e
        ))
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list_installed_profile_names() -> Result<Vec<String>> {
    let dir = apis_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .map(|ext| ext == "pb")
            .unwrap_or(false)
        {
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_home<F: FnOnce()>(home: &Path, f: F) {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HOME", home);
        f();
        std::env::remove_var("HOME");
    }

    #[test]
    fn test_not_installed_message_with_and_without_yaml() {
        let dir = tempfile::tempdir().unwrap();
        with_home(dir.path(), || {
            let msg = not_installed_message("missing");
            assert!(msg.contains("not installed"));
            assert!(msg.contains("<path/to/profile.yaml>"));

            let apis = apis_dir();
            std::fs::create_dir_all(&apis).unwrap();
            std::fs::write(apis.join("hint.yaml"), "name: hint\nversion: \"1\"\nbase_url: http://x\ncommands: {}")
                .unwrap();
            // name mismatch — still generic hint without yaml path for wrong name
            let msg2 = not_installed_message("other");
            assert!(msg2.contains("<path/to/profile.yaml>"));
        });
    }

    #[test]
    fn test_not_installed_message_points_at_yaml_source() {
        let dir = tempfile::tempdir().unwrap();
        let apis = dir.path().join(".config").join("ycallr").join("apis");
        std::fs::create_dir_all(&apis).unwrap();
        std::fs::write(apis.join("demo.yaml"), "name: demo\nversion: \"1\"\nbase_url: http://x\ncommands: {}")
            .unwrap();

        with_home(dir.path(), || {
            let msg = not_installed_message("demo");
            assert!(msg.contains("ycallr install"));
            assert!(msg.contains("demo.yaml"));
        });
    }

    #[test]
    fn test_list_installed_profile_names() {
        let dir = tempfile::tempdir().unwrap();
        with_home(dir.path(), || {
            assert!(list_installed_profile_names().unwrap().is_empty());

            let source = dir.path().join("alpha.yaml");
            std::fs::write(
                &source,
                r#"
name: alpha
version: "1"
base_url: http://127.0.0.1:9
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
            )
            .unwrap();
            install_profile_from_path(&source).unwrap();

            let names = list_installed_profile_names().unwrap();
            assert_eq!(names, ["alpha"]);
        });
    }

    #[test]
    fn test_install_profile_missing_yaml() {
        let dir = tempfile::tempdir().unwrap();
        with_home(dir.path(), || {
            let err = install_profile("ghost").unwrap_err();
            assert!(err.to_string().contains("YAML profile not found"));
        });
    }

    #[test]
    fn test_install_profile_from_path_rejects_non_yaml() {
        let dir = tempfile::tempdir().unwrap();
        with_home(dir.path(), || {
            let bad = dir.path().join("notes.txt");
            std::fs::write(&bad, "not yaml").unwrap();
            let err = install_profile_from_path(&bad).unwrap_err();
            assert!(err.to_string().contains("Expected a .yaml or .yml file"));
        });
    }

    #[test]
    fn test_load_installed_profile_corrupt_pb() {
        let dir = tempfile::tempdir().unwrap();
        let apis = dir.path().join(".config").join("ycallr").join("apis");
        std::fs::create_dir_all(&apis).unwrap();
        std::fs::write(apis.join("broken.pb"), b"not-protobuf").unwrap();

        with_home(dir.path(), || {
            let err = load_installed_profile("broken").unwrap_err();
            assert!(err.to_string().contains("Failed to decode"));
        });
    }

    #[test]
    fn test_resolve_yaml_source_path_yml_extension() {
        let dir = tempfile::tempdir().unwrap();
        let yml = dir.path().join("profile.yml");
        std::fs::write(&yml, "name: x\nversion: \"1\"\nbase_url: http://x\ncommands: {}").unwrap();
        let stem = dir.path().join("profile");
        let resolved = resolve_yaml_source_path(&stem).unwrap();
        assert_eq!(resolved, yml);
    }

    #[test]
    fn test_compile_yaml_str_roundtrip() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.example.com
commands:
  ping:
    endpoint: /ping
    method: GET
"#;
        let bytes = compile_yaml_str(yaml).unwrap();
        let api = load_from_proto_bytes(&bytes).unwrap();
        assert_eq!(api.name, "test");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_install_and_load_profile() {
        let dir = tempfile::tempdir().unwrap();
        let apis = dir.path().join(".config").join("ycallr").join("apis");
        std::fs::create_dir_all(&apis).unwrap();
        let yaml = apis.join("demo.yaml");
        std::fs::write(
            &yaml,
            r#"
name: demo
version: "1"
base_url: http://127.0.0.1:9
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
        )
        .unwrap();

        with_home(dir.path(), || {
            let pb = install_profile("demo").unwrap();
            assert!(pb.is_file());
            let api = load_installed_profile("demo").unwrap();
            assert_eq!(api.name, "demo");
            let err = load_installed_profile("missing").unwrap_err();
            assert!(matches!(err, YcallrError::ProfileNotInstalled { .. }));
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_install_profile_from_path_uses_yaml_name() {
        let dir = tempfile::tempdir().unwrap();
        let apis = dir.path().join(".config").join("ycallr").join("apis");
        std::fs::create_dir_all(&apis).unwrap();
        let source = dir.path().join("github_api.yaml");
        std::fs::write(
            &source,
            r#"
name: github
version: "1"
base_url: http://127.0.0.1:9
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
        )
        .unwrap();

        with_home(dir.path(), || {
            let (name, pb_path) = install_profile_from_path(&source).unwrap();
            assert_eq!(name, "github");
            assert_eq!(pb_path, apis.join("github.pb"));
            assert!(apis.join("github.yaml").is_file());
            assert!(!apis.join("github_api.pb").exists());
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_resolve_yaml_source_path_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = dir.path().join("profile.yaml");
        std::fs::write(&yaml, "name: x\nversion: \"1\"\nbase_url: http://x\ncommands: {}").unwrap();
        let stem = dir.path().join("profile");
        let resolved = resolve_yaml_source_path(&stem).unwrap();
        assert_eq!(resolved, yaml);
    }
}

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
    let mut msg = format!(
        "API profile '{}' is not installed. Install it with: ycallr install {}",
        name, name
    );
    if yaml_path.is_file() {
        msg.push_str(&format!(
            "\nYAML source found at {} — run install to compile it.",
            yaml_path.display()
        ));
    }
    msg
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

/// Install from an arbitrary YAML file (copies into apis dir when path differs).
#[cfg(not(target_arch = "wasm32"))]
pub fn install_profile_from_path(source: &Path) -> Result<(String, PathBuf)> {
    ensure_apis_dir()?;

    if !source.is_file() {
        return Err(YcallrError::ProfileInstall(format!(
            "YAML file not found: {}",
            source.display()
        )));
    }

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

    let name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            YcallrError::ProfileInstall(format!("Invalid file name: {}", source.display()))
        })?;

    let dest_yaml = yaml_profile_path(&name);
    if source.canonicalize().ok() != dest_yaml.canonicalize().ok() {
        std::fs::copy(source, &dest_yaml).map_err(|e| {
            YcallrError::ProfileInstall(format!(
                "Failed to copy YAML to {}: {}",
                dest_yaml.display(),
                e
            ))
        })?;
    }

    let pb_path = install_profile_yaml_path(&name, &dest_yaml)?;
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

        std::env::set_var("HOME", dir.path());
        let pb = install_profile("demo").unwrap();
        assert!(pb.is_file());
        let api = load_installed_profile("demo").unwrap();
        assert_eq!(api.name, "demo");
        let err = load_installed_profile("missing").unwrap_err();
        assert!(matches!(err, YcallrError::ProfileNotInstalled { .. }));
        std::env::remove_var("HOME");
    }
}

#![cfg(all(not(target_arch = "wasm32"), feature = "ffi"))]

use std::ffi::CString;
use std::ptr;
use ycallr_core::ffi::{
    ycallr_free_api, ycallr_get_base_url, ycallr_get_name, ycallr_get_version, ycallr_parse_yaml,
};

const VALID_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub API
base_url: https://api.github.com
commands:
  get-repo:
    endpoint: /repos/{owner}/{repo}
    method: GET
    headers:
      Accept: application/json
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
"#;

#[test]
fn test_ffi_parse_valid_yaml() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    unsafe {
        let name = ycallr_get_name(api);
        assert!(!name.is_null());
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap();
        assert_eq!(name_str, "github");

        let version = ycallr_get_version(api);
        assert!(!version.is_null());
        let version_str = std::ffi::CStr::from_ptr(version).to_str().unwrap();
        assert_eq!(version_str, "1.0.0");

        let base_url = ycallr_get_base_url(api);
        assert!(!base_url.is_null());
        let base_url_str = std::ffi::CStr::from_ptr(base_url).to_str().unwrap();
        assert_eq!(base_url_str, "https://api.github.com");

        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_parse_null_yaml() {
    let api = unsafe { ycallr_parse_yaml(ptr::null()) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_parse_invalid_yaml() {
    let yaml = CString::new("not valid yaml {{{").unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_parse_invalid_utf8() {
    let invalid_utf8 = &[0xFF, 0xFE, 0x00];
    let api = unsafe { ycallr_parse_yaml(invalid_utf8.as_ptr() as *const i8) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_free_null() {
    unsafe { ycallr_free_api(ptr::null_mut()) };
}

#[test]
fn test_ffi_getters_null() {
    unsafe {
        assert!(ycallr_get_name(ptr::null()).is_null());
        assert!(ycallr_get_version(ptr::null()).is_null());
        assert!(ycallr_get_base_url(ptr::null()).is_null());
    }
}

#[test]
fn test_ffi_parse_validation_error() {
    let yaml = CString::new(
        r#"
name: ""
version: "1.0.0"
base_url: https://api.test.com
commands: {}
"#,
    )
    .unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(api.is_null());
}

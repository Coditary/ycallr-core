use std::os::raw::c_char;

#[repr(C)]
pub struct YcallrApi {
    name: *mut c_char,
    version: *mut c_char,
    base_url: *mut c_char,
}

#[no_mangle]
pub extern "C" fn ycallr_parse_yaml(yaml: *const c_char) -> *mut YcallrApi {
    if yaml.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(yaml) };

    let yaml_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let api = match crate::yaml_parser::parse_yaml(yaml_str) {
        Ok(a) => a,
        Err(_) => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(YcallrApi {
        name: into_raw_cstring(api.name),
        version: into_raw_cstring(api.version),
        base_url: into_raw_cstring(api.base_url),
    }))
}

#[no_mangle]
pub extern "C" fn ycallr_free_api(api: *mut YcallrApi) {
    if !api.is_null() {
        unsafe {
            let _ = from_raw_cstring((*api).name);
            let _ = from_raw_cstring((*api).version);
            let _ = from_raw_cstring((*api).base_url);
            let _ = Box::from_raw(api);
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_get_name(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).name }
}

#[no_mangle]
pub extern "C" fn ycallr_get_version(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).version }
}

#[no_mangle]
pub extern "C" fn ycallr_get_base_url(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).base_url }
}

fn into_raw_cstring(s: String) -> *mut c_char {
    std::ffi::CString::new(s).unwrap().into_raw()
}

unsafe fn from_raw_cstring(ptr: *mut c_char) -> std::ffi::CString {
    std::ffi::CString::from_raw(ptr)
}

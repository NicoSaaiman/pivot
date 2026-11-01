use std::ffi::{c_char, CString};

use shared::models::Process;

use crate::errors::Result;

unsafe extern "C" {
    fn ffi_print_process_details(
        command: *const c_char,
        args: *const *const c_char,
        argc: usize,
        env_keys: *const *const c_char,
        env_values: *const *const c_char,
        env_count: usize,
        working_dir: *const c_char,
    );
}

pub fn print_process_details(process: &Process) -> Result<()> {
    let command = CString::new(process.get_command().as_bytes())?;

    let args: Vec<CString> = process
        .get_args()
        .iter()
        .map(|arg| CString::new(arg.as_str()))
        .collect::<std::result::Result<_, _>>()?;

    let args_ptrs: Vec<*const c_char> = args
        .iter()
        .map(|arg| arg.as_ptr())
        .collect();

    let env: Vec<(CString, CString)> = process
        .get_env()
        .iter()
        .map(|(key, value)| {
            Ok((CString::new(key.as_str())?, CString::new(value.as_str())?))
        })
        .collect::<std::result::Result<_, std::ffi::NulError>>()?;

    let env_keys: Vec<*const c_char> = env
        .iter()
        .map(|(key, _)| key.as_ptr())
        .collect();

    let env_values: Vec<*const c_char> = env
        .iter()
        .map(|(_, value)| value.as_ptr())
        .collect();

    let working_dir = process
        .get_working_dir()
        .map(CString::new)
        .transpose()?;

    let working_dir_ptr = working_dir
        .as_ref()
        .map_or(std::ptr::null(), |dir| dir.as_ptr());

    unsafe {
        ffi_print_process_details(
            command.as_ptr(),
            args_ptrs.as_ptr(),
            args_ptrs.len(),
            env_keys.as_ptr(),
            env_values.as_ptr(),
            env_keys.len(),
            working_dir_ptr,
        );
    }

    Ok(())
}

use std::ffi::{c_char, c_int, CString};

use shared::models::Process;

use crate::errors::{Result, RuntimeError};

struct OwnedCProcess {
    command: CString,
    args: Vec<CString>,
    args_ptrs: Vec<*const c_char>,
    env_keys: Vec<CString>,
    env_keys_ptrs: Vec<*const c_char>,
    env_values: Vec<CString>,
    env_values_ptrs: Vec<*const c_char>,
    working_dir: Option<CString>,
}

impl OwnedCProcess {
    fn convert(process: &Process) -> Result<Self> {
        let command = CString::new(process.get_command())?;

        let args: Vec<CString> = process
            .get_args()
            .iter()
            .map(|arg| CString::new(arg.as_str()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        
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
            .collect::<std::result::Result<Vec<_>, std::ffi::NulError>>()?;

        let (env_keys, env_values): (Vec<_>, Vec<_>) =
            env.into_iter().unzip();

        let env_keys_ptrs: Vec<*const c_char> = env_keys
            .iter()
            .map(|key| key.as_ptr())
            .collect();

        let env_values_ptrs: Vec<*const c_char> = env_values
            .iter()
            .map(|value| value.as_ptr())
            .collect();

        let working_dir = process
            .get_working_dir()
            .map(CString::new)
            .transpose()?;

        
        Ok(
            Self {
                command,
                args,
                args_ptrs,
                env_keys,
                env_keys_ptrs,
                env_values,
                env_values_ptrs,
                working_dir,
        })
    }

    fn as_ffi(&self) -> CProcess {
        CProcess {
            command: self.command.as_ptr(),
            args: self.args_ptrs.as_ptr(),
            argc: self.args_ptrs.len(),
            env_keys: self.env_keys_ptrs.as_ptr(),
            env_values: self.env_values_ptrs.as_ptr(),
            env_count: self.env_keys_ptrs.len(),
            working_dir: self.working_dir.as_ref().map_or(std::ptr::null(), |dir| dir.as_ptr()),
        }
    }
}

#[repr(C)]
struct CProcess {
    command: *const c_char,
    args: *const *const c_char,
    argc: usize,
    env_keys: *const *const c_char,
    env_values: *const *const c_char,
    env_count: usize,
    working_dir: *const c_char,

}

unsafe extern "C" {
    fn ffi_print_process_details(process: *const CProcess);
    fn ffi_create_process(process: *const CProcess) -> c_int;
}

pub fn print_process_details(process: &Process) -> Result<()> {
    let owned = OwnedCProcess::convert(process)?;
    let ffi_process = owned.as_ffi();

    unsafe {
        ffi_print_process_details(&ffi_process);
    }

    Ok(())
}

pub fn create_process(process: &Process) -> Result<c_int> {
    let owned = OwnedCProcess::convert(process)?;
    let ffi_process = owned.as_ffi();

    let pid = unsafe {
        ffi_create_process(&ffi_process)
    };

    if pid == -1 {
        return Err(RuntimeError::ForkFailed);
    }

    Ok(pid)
}

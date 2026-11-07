use std::ffi::{c_char, c_int, CString};

use shared::models::{ExecutionState, Process};

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
    fn ffi_create_process(process: *const CProcess) -> c_int;
    fn ffi_get_process_state(pid: c_int) -> c_int;
    fn ffi_terminate_process(pid: c_int) -> c_int;
    fn ffi_wait_process_blocking(pid: c_int) -> c_int;
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

pub fn get_process_state(pid: c_int) -> Result<ExecutionState> {
    let state = unsafe {
        ffi_get_process_state(pid)
    };

    let state = match state {
        0 => ExecutionState::Running,
        1 | 2 => ExecutionState::Exited,
        -1 => return Err(RuntimeError::ProcessStateFailed),
        _ => ExecutionState::Unknown,
    };

    Ok(state)
}

pub fn terminate_process(pid: c_int) -> Result<()> {
    let result = unsafe {
        ffi_terminate_process(pid)
    };

    if result == -1 {
        return Err(RuntimeError::TerminationFailed);
    }

    Ok(())
}

pub fn wait_process_blocking(pid: c_int) -> Result<()> {
    let result = unsafe {
        ffi_wait_process_blocking(pid)
    };

    if result == -1 {
        return Err(RuntimeError::WaitFailed);
    }

    Ok(()) 
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::collections::BTreeMap;

    #[test]
    fn owned_c_process_convert_success() {
        let mut env = BTreeMap::new();
        env.insert("APP_ENV".to_string(), "test".to_string());
        env.insert("LOG_LEVEL".to_string(), "debug".to_string());

        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec!["--verbose".to_string(), "--port".to_string(), "8080".to_string()],
            env,
            Some("/tmp".to_string()),
        );

        let owned = OwnedCProcess::convert(&process).unwrap();

        assert_eq!(owned.command.to_str().unwrap(), "/usr/bin/my-app");
        assert_eq!(owned.args.len(), 3);
        assert_eq!(owned.args[0].to_str().unwrap(), "--verbose");
        assert_eq!(owned.args[1].to_str().unwrap(), "--port");
        assert_eq!(owned.args[2].to_str().unwrap(), "8080");

        assert_eq!(owned.env_keys.len(), 2);
        assert_eq!(owned.env_values.len(), 2);
        assert_eq!(owned.working_dir.as_ref().unwrap().to_str().unwrap(), "/tmp");
    }

    #[test]
    fn owned_c_process_convert_invalid_command() {
        let process = Process::new(
            "invalid\0command".to_string(),
            vec![],
            BTreeMap::new(),
            None,
        );

        let result = OwnedCProcess::convert(&process);

        assert!(result.is_err());
    }

    #[test]
    fn owned_c_process_convert_invalid_argument() {
        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec!["valid".to_string(), "invalid\0argument".to_string()],
            BTreeMap::new(),
            None,
        );

        let result = OwnedCProcess::convert(&process);

        assert!(result.is_err());
    }

    #[test]
    fn owned_c_process_convert_invalid_env_key() {
        let mut env = BTreeMap::new();
        env.insert("INVALID\0KEY".to_string(), "value".to_string());

        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec![],
            env,
            None,
        );

        let result = OwnedCProcess::convert(&process);

        assert!(result.is_err());
    }

    #[test]
    fn owned_c_process_convert_invalid_env_value() {
        let mut env = BTreeMap::new();
        env.insert("VALID_KEY".to_string(), "INVALID\0VALUE".to_string());

        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec![],
            env,
            None,
        );

        let result = OwnedCProcess::convert(&process);

        assert!(result.is_err());
    }

    #[test]
    fn owned_c_process_convert_invalid_working_dir() {
        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec![],
            BTreeMap::new(),
            Some("invalid\0directory".to_string()),
        );

        let result = OwnedCProcess::convert(&process);

        assert!(result.is_err());
    }

    #[test]
    fn owned_c_process_convert_minimal_process() {
        let process = Process::new(
            "/usr/bin/my-app".to_string(),
            vec![],
            BTreeMap::new(),
            None,
        );

        let owned = OwnedCProcess::convert(&process).unwrap();
        let ffi_process = owned.as_ffi();
        
        assert_eq!(ffi_process.argc, 0);
        assert_eq!(ffi_process.env_count, 0);
        assert!(ffi_process.working_dir.is_null());
    }

    #[test]
    fn get_process_state_invalid_pid() {
        let result = get_process_state(-1);

        assert!(matches!(
            result,
            Err(RuntimeError::ProcessStateFailed)
        ));
    }

    #[test]
    fn terminate_process_invalid_pid() {
        let result = terminate_process(i32::MAX);

        assert!(matches!(
            result,
            Err(RuntimeError::TerminationFailed)
        ));
    }

    #[test]
    fn wait_process_blocking_invalid_pid() {
        let result = wait_process_blocking(i32::MAX);

        assert!(matches!(
            result,
            Err(RuntimeError::WaitFailed)
        ));
    }
}

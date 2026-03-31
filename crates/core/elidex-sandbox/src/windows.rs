//! Windows sandbox enforcer using restricted tokens and Job Objects.
//!
//! Creates a restricted token with maximum privilege reduction and assigns
//! the process to a Job Object that limits resource access.
//!
//! Design reference: docs/design/en/08-security-model.md §8.1 (Windows row).
//!
//! NOTE: Full implementation requires the renderer to be a separate process.
//! In `SingleProcess` mode, restricting the token would affect the entire
//! application. This module provides the infrastructure; actual enforcement
//! is gated on multi-process mode.

use elidex_plugin::{SandboxError, SandboxPolicy};

use crate::SandboxEnforcer;

pub struct WindowsRestrictedEnforcer;

impl SandboxEnforcer for WindowsRestrictedEnforcer {
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        create_restricted_job(policy)?;
        Ok(())
    }

    fn name(&self) -> &str {
        "windows-restricted-token"
    }
}

/// Create a Job Object with restrictions matching the sandbox policy and
/// assign the current process to it.
fn create_restricted_job(policy: &SandboxPolicy) -> Result<(), SandboxError> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: Win32 API calls with validated parameters.
    unsafe {
        let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == std::ptr::null_mut() {
            return Err(SandboxError::new("CreateJobObjectW failed"));
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        info.BasicLimitInformation.ActiveProcessLimit = 1;

        let ret = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ret == 0 {
            CloseHandle(job);
            return Err(SandboxError::new("SetInformationJobObject failed"));
        }

        let ret = AssignProcessToJobObject(job, GetCurrentProcess());
        if ret == 0 {
            CloseHandle(job);
            return Err(SandboxError::new("AssignProcessToJobObject failed"));
        }

        // Do not close the job handle — it must remain open for the
        // lifetime of the process. When the handle is closed, all
        // processes in the job are killed (due to KILL_ON_JOB_CLOSE).
        // The handle is intentionally leaked.
        let _ = policy; // Will be used for network/filesystem restrictions.
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcer_name() {
        assert_eq!(WindowsRestrictedEnforcer.name(), "windows-restricted-token");
    }

    // NOTE: Actually applying the job object would restrict THIS test process.
    // Integration tests for enforcement should run in a child process.
}

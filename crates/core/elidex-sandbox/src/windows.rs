//! Windows sandbox enforcer using Job Objects.
//!
//! Assigns the process to a Job Object that limits the process count. It does
//! not create a restricted token and does not enforce the policy's filesystem
//! or network restrictions (`#11-os-sandbox-enforcer-correctness`; the shell
//! stays on `PlatformSandbox::Unsandboxed` until that slot's S1–S6 close).
//!
//! Design reference: docs/design/en/08-security-model.md §8.1 (Windows row).
//!
//! NOTE: Enforcement requires the renderer to be a separate process. In
//! `SingleProcess` mode, restricting the process would affect the entire
//! application.

use elidex_plugin::{SandboxError, SandboxPolicy};

use crate::SandboxEnforcer;

pub struct WindowsRestrictedEnforcer;

impl SandboxEnforcer for WindowsRestrictedEnforcer {
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        create_restricted_job(policy)
    }

    fn name(&self) -> &'static str {
        "windows-restricted-token"
    }
}

/// Create a Job Object with restrictions matching the sandbox policy and
/// assign the current process to it.
///
/// Defect: only the job's process-count limit is applied. The policy's filesystem
/// and network restrictions are not enforced on Windows, and `apply` still
/// returns `Ok` (`#11-os-sandbox-enforcer-correctness`).
#[allow(unsafe_code)]
fn create_restricted_job(_policy: &SandboxPolicy) -> Result<(), SandboxError> {
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
        if job.is_null() {
            return Err(SandboxError::new("CreateJobObjectW failed"));
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        info.BasicLimitInformation.ActiveProcessLimit = 1;

        let ret = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&raw const info).cast(),
            u32::try_from(std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                .expect("JOBOBJECT_EXTENDED_LIMIT_INFORMATION size fits in u32"),
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

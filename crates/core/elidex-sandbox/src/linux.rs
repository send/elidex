//! Linux seccomp-bpf sandbox enforcer.
//!
//! Uses `seccompiler` to install a BPF filter that restricts the system call
//! surface of the calling thread. Once installed, the filter is inherited by
//! child threads and **cannot be removed**.
//!
//! Design reference: docs/design/en/08-security-model.md §8.1 (Linux row).

use std::collections::BTreeMap;

use elidex_plugin::sandbox::NetworkAccess;
use elidex_plugin::{SandboxError, SandboxPolicy};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompFilter, SeccompRule, TargetArch,
};

use crate::SandboxEnforcer;

pub struct LinuxSeccompEnforcer;

impl SandboxEnforcer for LinuxSeccompEnforcer {
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        let filter = build_filter(policy).map_err(|e| SandboxError::new(format!("{e:#}")))?;
        let bpf: BpfProgram =
            filter.try_into().map_err(|e: seccompiler::Error| SandboxError::new(format!("{e}")))?;

        seccompiler::apply_filter(&bpf)
            .map_err(|e| SandboxError::new(format!("seccomp install failed: {e}")))?;

        Ok(())
    }

    fn name(&self) -> &str {
        "linux-seccomp-bpf"
    }
}

/// Build a seccomp filter from the sandbox policy.
///
/// The default action is `Errno(EPERM)` — blocked syscalls return permission
/// denied rather than killing the process, allowing graceful error handling.
fn build_filter(policy: &SandboxPolicy) -> Result<SeccompFilter, seccompiler::Error> {
    let arch = target_arch();

    // Map of syscall number → rules (empty Vec = unconditionally allow).
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // Always allow: memory, signals, time, threading, IPC.
    let always_allow = [
        libc::SYS_read,
        libc::SYS_readv,
        libc::SYS_write,
        libc::SYS_writev,
        libc::SYS_close,
        libc::SYS_mmap,
        libc::SYS_munmap,
        libc::SYS_mprotect,
        libc::SYS_madvise,
        libc::SYS_brk,
        libc::SYS_futex,
        libc::SYS_clock_gettime,
        libc::SYS_clock_getres,
        libc::SYS_gettimeofday,
        libc::SYS_nanosleep,
        libc::SYS_clock_nanosleep,
        libc::SYS_sigaltstack,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_exit,
        libc::SYS_exit_group,
        libc::SYS_sched_yield,
        libc::SYS_sched_getaffinity,
        libc::SYS_getpid,
        libc::SYS_gettid,
        libc::SYS_tgkill,
        libc::SYS_poll,
        libc::SYS_epoll_create1,
        libc::SYS_epoll_ctl,
        libc::SYS_epoll_wait,
        libc::SYS_eventfd2,
        libc::SYS_pipe2,
        libc::SYS_getrandom,
        libc::SYS_mremap,
        libc::SYS_dup,
        libc::SYS_dup2,
        libc::SYS_fcntl,
        libc::SYS_ioctl,
        libc::SYS_lseek,
        libc::SYS_fstat,
        #[cfg(target_arch = "x86_64")]
        libc::SYS_newfstatat,
        #[cfg(target_arch = "aarch64")]
        libc::SYS_newfstatat,
        libc::SYS_pread64,
        libc::SYS_pwrite64,
        libc::SYS_clone,
        libc::SYS_set_robust_list,
        libc::SYS_rseq,
        libc::SYS_membarrier,
        libc::SYS_prctl,
    ];

    for &syscall in &always_allow {
        rules.insert(syscall, vec![]);
    }

    // IPC: socketpair for in-process channel communication.
    if policy.ipc {
        rules.insert(libc::SYS_socketpair, vec![]);
        rules.insert(libc::SYS_sendmsg, vec![]);
        rules.insert(libc::SYS_recvmsg, vec![]);
        rules.insert(libc::SYS_sendto, vec![]);
        rules.insert(libc::SYS_recvfrom, vec![]);
    }

    // Network: connect/bind/listen only if network access is not None.
    if policy.network != NetworkAccess::None {
        rules.insert(libc::SYS_socket, vec![]);
        rules.insert(libc::SYS_connect, vec![]);
        rules.insert(libc::SYS_sendto, vec![]);
        rules.insert(libc::SYS_recvfrom, vec![]);
        rules.insert(libc::SYS_getsockopt, vec![]);
        rules.insert(libc::SYS_setsockopt, vec![]);
        rules.insert(libc::SYS_getpeername, vec![]);
        rules.insert(libc::SYS_getsockname, vec![]);
        rules.insert(libc::SYS_shutdown, vec![]);
    }

    // Filesystem: open/openat only if filesystem access is not None.
    if policy.filesystem != elidex_plugin::sandbox::FilesystemAccess::None {
        rules.insert(libc::SYS_openat, vec![]);
        rules.insert(libc::SYS_access, vec![]);
        rules.insert(libc::SYS_stat, vec![]);
        rules.insert(libc::SYS_readlink, vec![]);
        rules.insert(libc::SYS_getcwd, vec![]);

        if policy.filesystem == elidex_plugin::sandbox::FilesystemAccess::ReadWrite {
            rules.insert(libc::SYS_unlink, vec![]);
            rules.insert(libc::SYS_rename, vec![]);
            rules.insert(libc::SYS_mkdir, vec![]);
            rules.insert(libc::SYS_rmdir, vec![]);
            rules.insert(libc::SYS_ftruncate, vec![]);
            rules.insert(libc::SYS_fallocate, vec![]);
            rules.insert(libc::SYS_fsync, vec![]);
            rules.insert(libc::SYS_fdatasync, vec![]);
        }
    }

    // Blocked by default (never allowed for Renderer):
    // - execve, fork (process creation)
    // - bind, listen, accept (server sockets)
    // - ptrace (debugging)
    // - mount, umount (filesystem)
    // - reboot, init_module, etc. (admin)

    SeccompFilter::new(
        rules,
        SeccompAction::Errno(libc::EPERM as u32),
        SeccompAction::Allow,
        arch,
    )
}

fn target_arch() -> TargetArch {
    #[cfg(target_arch = "x86_64")]
    {
        TargetArch::x86_64
    }
    #[cfg(target_arch = "aarch64")]
    {
        TargetArch::aarch64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::SandboxPolicy;

    #[test]
    fn build_strict_filter() {
        let filter = build_filter(&SandboxPolicy::strict());
        assert!(filter.is_ok());
    }

    #[test]
    fn build_web_content_filter() {
        let filter = build_filter(&SandboxPolicy::web_content());
        assert!(filter.is_ok());
    }

    #[test]
    fn build_permissive_filter() {
        let filter = build_filter(&SandboxPolicy::permissive());
        assert!(filter.is_ok());
    }

    #[test]
    fn enforcer_name() {
        assert_eq!(LinuxSeccompEnforcer.name(), "linux-seccomp-bpf");
    }

    // NOTE: Actually applying the filter would restrict THIS test process.
    // Integration tests for enforcement should run in a child process.
}

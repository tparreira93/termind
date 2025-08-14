use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[derive(Debug, Clone, PartialEq)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
    Running,
    Stopped(i32),
}

impl ExitStatus {
    pub fn success(&self) -> bool {
        matches!(self, ExitStatus::Code(0))
    }
    
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            ExitStatus::Code(code) => Some(*code),
            _ => None,
        }
    }
    
    pub fn signal(&self) -> Option<i32> {
        match self {
            ExitStatus::Signal(sig) => Some(*sig),
            _ => None,
        }
    }
}

pub struct ProcessManager {
    child_pid: Pid,
}

impl ProcessManager {
    pub fn new(child_pid: Pid) -> Self {
        Self { child_pid }
    }
    
    /// Wait for the child process to exit (non-blocking check)
    pub fn try_wait(&self) -> Result<ExitStatus, nix::Error> {
        match waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => Ok(ExitStatus::Running),
            Ok(WaitStatus::Exited(_, code)) => {
                info!("Child process {} exited with code {}", self.child_pid, code);
                Ok(ExitStatus::Code(code))
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                info!("Child process {} terminated by signal {}", self.child_pid, signal as i32);
                Ok(ExitStatus::Signal(signal as i32))
            }
            Ok(WaitStatus::Stopped(_, signal)) => {
                debug!("Child process {} stopped by signal {}", self.child_pid, signal as i32);
                Ok(ExitStatus::Stopped(signal as i32))
            }
            Ok(WaitStatus::Continued(_)) => {
                debug!("Child process {} continued", self.child_pid);
                Ok(ExitStatus::Running)
            }
            // All WaitStatus variants are explicitly handled above
            #[allow(unreachable_patterns)]
            Ok(_) => {
                // This should never be reached as all WaitStatus variants are covered
                Ok(ExitStatus::Running)
            }
            Err(nix::Error::ECHILD) => {
                // Child already reaped
                debug!("Child process {} already reaped", self.child_pid);
                Ok(ExitStatus::Code(0)) // Assume success if we can't get status
            }
            Err(e) => {
                error!("Error waiting for child {}: {}", self.child_pid, e);
                Err(e)
            }
        }
    }
    
    /// Wait for the child process to exit (blocking)
    pub async fn wait_for_exit(&self) -> Result<ExitStatus, nix::Error> {
        loop {
            match self.try_wait()? {
                ExitStatus::Running => {
                    sleep(Duration::from_millis(10)).await;
                    continue;
                }
                status => return Ok(status),
            }
        }
    }
    
    /// Wait for the child process to exit with a timeout
    pub async fn wait_for_exit_timeout(&self, timeout: Duration) -> Result<Option<ExitStatus>, nix::Error> {
        let start = std::time::Instant::now();
        
        loop {
            match self.try_wait()? {
                ExitStatus::Running => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    sleep(Duration::from_millis(10)).await;
                    continue;
                }
                status => return Ok(Some(status)),
            }
        }
    }
    
    /// Check if the child process is still running
    pub fn is_running(&self) -> Result<bool, nix::Error> {
        match self.try_wait()? {
            ExitStatus::Running => Ok(true),
            _ => Ok(false),
        }
    }
    
    /// Get the child process ID
    pub fn pid(&self) -> Pid {
        self.child_pid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::unistd;
    
    #[test]
    fn test_process_manager_creation() {
        let pid = unistd::getpid();
        let manager = ProcessManager::new(pid);
        assert_eq!(manager.pid(), pid);
    }
    
    #[test]
    fn test_exit_status_methods() {
        assert!(ExitStatus::Code(0).success());
        assert!(!ExitStatus::Code(1).success());
        assert_eq!(ExitStatus::Code(42).exit_code(), Some(42));
        assert_eq!(ExitStatus::Signal(9).signal(), Some(9));
    }
}

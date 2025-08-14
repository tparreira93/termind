use std::env;
use std::ffi::CString;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;

use nix::fcntl::OFlag;
use nix::pty::{self, PtyMaster};
use nix::unistd::{self, ForkResult, Pid};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("PTY creation failed: {0}")]
    PtyCreation(#[from] nix::Error),
    
    #[error("Fork failed: {0}")]
    Fork(String),
    
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Shell not found: {0}")]
    ShellNotFound(String),
    
    #[error("Environment setup failed")]
    EnvironmentSetup,
}

pub struct PtyHost {
    master: PtyMaster,
    child_pid: Pid,
    reader: tokio::fs::File,
    writer: tokio::fs::File,
    shell_path: String,
}

impl PtyHost {
    /// Spawn a new shell process with PTY
    pub async fn spawn_shell() -> Result<Self, PtyError> {
        let shell_path = Self::detect_shell()?;
        info!("Spawning shell: {}", shell_path);
        
        // Create PTY master/slave pair
        let master = pty::posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;
        pty::grantpt(&master)?;
        pty::unlockpt(&master)?;
        
        let slave_name = unsafe { pty::ptsname(&master)? };
        debug!("PTY slave created: {}", slave_name);
        
        // Fork the process
        match unsafe { unistd::fork()? } {
            ForkResult::Parent { child } => {
                info!("Forked child process: {}", child);
                Self::setup_parent(master, child, shell_path).await
            }
            ForkResult::Child => {
                // This code runs in the child process
                Self::setup_child(&slave_name, &shell_path).await
            }
        }
    }
    
    /// Setup parent process with async I/O
    async fn setup_parent(
        master: PtyMaster, 
        child_pid: Pid, 
        shell_path: String
    ) -> Result<Self, PtyError> {
        let master_fd = master.as_raw_fd();
        
        // Convert to tokio async files for reading and writing
        let reader = unsafe { 
            tokio::fs::File::from_raw_fd(libc::dup(master_fd))
        };
        let writer = unsafe { 
            tokio::fs::File::from_raw_fd(libc::dup(master_fd))
        };
        
        debug!("Parent process setup complete");
        
        Ok(Self {
            master,
            child_pid,
            reader,
            writer,
            shell_path,
        })
    }
    
    /// Setup child process to run the shell
    async fn setup_child(slave_name: &str, shell_path: &str) -> Result<Self, PtyError> {
        // This function never returns in the child process
        // It either execs successfully or exits with error
        
        // Create new session
        if let Err(e) = unistd::setsid() {
            error!("Failed to create new session: {}", e);
            std::process::exit(1);
        }
        
        // Open slave PTY
        let slave_fd = match nix::fcntl::open(
            slave_name,
            nix::fcntl::OFlag::O_RDWR,
            nix::sys::stat::Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(e) => {
                error!("Failed to open slave PTY: {}", e);
                std::process::exit(1);
            }
        };
        
        // Redirect stdin, stdout, stderr to slave
        for fd in &[0, 1, 2] {
            if let Err(e) = unistd::dup2(slave_fd, *fd) {
                error!("Failed to redirect fd {}: {}", fd, e);
                std::process::exit(1);
            }
        }
        
        // Close the original slave fd
        if let Err(e) = unistd::close(slave_fd) {
            error!("Failed to close slave fd: {}", e);
        }
        
        // Set controlling terminal
        unsafe {
            if libc::ioctl(0, libc::TIOCSCTTY as libc::c_ulong, 0) < 0 {
                error!("Failed to set controlling terminal");
                std::process::exit(1);
            }
        }
        
        // Setup environment
        env::set_var("TERM", "xterm-256color");
        if let Some(home) = dirs::home_dir() {
            env::set_var("HOME", home);
        }
        
        // Execute the shell
        let shell_cstring = CString::new(shell_path).unwrap();
        let shell_arg = CString::new(shell_path).unwrap();
        
        info!("Child: exec shell {}", shell_path);
        
        match unistd::execv(&shell_cstring, &[shell_arg]) {
            Err(e) => {
                error!("Failed to exec shell: {}", e);
                std::process::exit(1);
            }
            Ok(_) => {
                // This should never be reached
                unreachable!("execv returned Ok");
            }
        }
    }
    
    /// Detect the user's preferred shell
    fn detect_shell() -> Result<String, PtyError> {
        // Try SHELL environment variable first
        if let Ok(shell) = env::var("SHELL") {
            if Path::new(&shell).exists() {
                return Ok(shell);
            }
        }
        
        // Try common shell locations
        let shells = [
            "/bin/zsh",
            "/usr/bin/zsh", 
            "/bin/bash",
            "/usr/bin/bash",
            "/bin/sh",
        ];
        
        for shell in &shells {
            if Path::new(shell).exists() {
                return Ok(shell.to_string());
            }
        }
        
        Err(PtyError::ShellNotFound("No suitable shell found".to_string()))
    }
    
    /// Resize the PTY
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), PtyError> {
        debug!("Resizing PTY to {}x{}", cols, rows);
        
        let winsize = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        
        unsafe {
            if libc::ioctl(
                self.master.as_raw_fd(),
                libc::TIOCSWINSZ,
                &winsize as *const _
            ) < 0 {
                return Err(PtyError::Io(io::Error::last_os_error()));
            }
        }
        
        // Send SIGWINCH to child to notify of resize
        if let Err(e) = nix::sys::signal::kill(self.child_pid, nix::sys::signal::SIGWINCH) {
            warn!("Failed to send SIGWINCH to child: {}", e);
        }
        
        Ok(())
    }
    
    /// Read data from PTY (non-blocking)
    pub async fn try_read(&mut self) -> Result<Vec<u8>, PtyError> {
        let mut buffer = vec![0u8; 4096];
        
        // Use a timeout for non-blocking behavior
        match tokio::time::timeout(std::time::Duration::from_millis(1), self.reader.read(&mut buffer)).await {
            Ok(Ok(0)) => Ok(Vec::new()),
            Ok(Ok(n)) => {
                buffer.truncate(n);
                Ok(buffer)
            }
            Ok(Err(e)) => Err(PtyError::Io(e)),
            Err(_) => Ok(Vec::new()), // Timeout = no data available
        }
    }
    
    /// Read data from PTY (blocking)
    pub async fn read(&mut self) -> Result<Vec<u8>, PtyError> {
        let mut buffer = vec![0u8; 4096];
        
        match self.reader.read(&mut buffer).await {
            Ok(0) => Ok(Vec::new()), // EOF
            Ok(n) => {
                buffer.truncate(n);
                Ok(buffer)
            }
            Err(e) => Err(PtyError::Io(e)),
        }
    }
    
    /// Write data to PTY
    pub async fn write(&mut self, data: &[u8]) -> Result<(), PtyError> {
        self.writer.write_all(data).await?;
        self.writer.flush().await?;
        Ok(())
    }
    
    /// Get child process ID
    pub fn child_pid(&self) -> Pid {
        self.child_pid
    }
    
    /// Get shell path
    pub fn shell_path(&self) -> &str {
        &self.shell_path
    }
}

impl Drop for PtyHost {
    fn drop(&mut self) {
        debug!("Dropping PtyHost, child_pid: {}", self.child_pid);
        
        // Send SIGTERM to child process
        if let Err(e) = nix::sys::signal::kill(self.child_pid, nix::sys::signal::SIGTERM) {
            warn!("Failed to send SIGTERM to child: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shell_detection() {
        let shell = PtyHost::detect_shell().unwrap();
        assert!(!shell.is_empty());
        assert!(Path::new(&shell).exists());
    }
    
    #[tokio::test]
    async fn test_pty_spawn() {
        let pty = PtyHost::spawn_shell().await;
        assert!(pty.is_ok());
    }
}

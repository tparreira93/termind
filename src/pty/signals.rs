use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error};

pub struct SignalHandler {
    sigint: tokio::signal::unix::Signal,
    sigterm: tokio::signal::unix::Signal,
    sigwinch: tokio::signal::unix::Signal,
    child_pid: Pid,
}

impl SignalHandler {
    pub fn new(child_pid: Pid) -> Result<Self, std::io::Error> {
        Ok(Self {
            sigint: signal(SignalKind::interrupt())?,
            sigterm: signal(SignalKind::terminate())?,
            sigwinch: signal(SignalKind::window_change())?,
            child_pid,
        })
    }
    
    /// Handle incoming signals and forward them to the child process
    pub async fn handle_signals(&mut self) -> SignalEvent {
        tokio::select! {
            _ = self.sigint.recv() => {
                debug!("Received SIGINT, forwarding to child {}", self.child_pid);
                if let Err(e) = signal::kill(self.child_pid, Signal::SIGINT) {
                    error!("Failed to forward SIGINT to child: {}", e);
                }
                SignalEvent::Interrupt
            }
            _ = self.sigterm.recv() => {
                debug!("Received SIGTERM, forwarding to child {}", self.child_pid);
                if let Err(e) = signal::kill(self.child_pid, Signal::SIGTERM) {
                    error!("Failed to forward SIGTERM to child: {}", e);
                }
                SignalEvent::Terminate
            }
            _ = self.sigwinch.recv() => {
                debug!("Received SIGWINCH");
                // Don't forward SIGWINCH - the terminal resize handler will do this
                SignalEvent::WindowChange
            }
        }
    }
    
    /// Send a specific signal to the child process
    pub fn send_signal(&self, sig: Signal) -> Result<(), nix::Error> {
        signal::kill(self.child_pid, sig)
    }
    
    /// Send SIGINT (Ctrl+C) to child
    pub fn interrupt(&self) -> Result<(), nix::Error> {
        self.send_signal(Signal::SIGINT)
    }
    
    /// Send SIGTERM to child
    pub fn terminate(&self) -> Result<(), nix::Error> {
        self.send_signal(Signal::SIGTERM)
    }
    
    /// Send SIGKILL to child (force kill)
    pub fn kill(&self) -> Result<(), nix::Error> {
        self.send_signal(Signal::SIGKILL)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalEvent {
    Interrupt,
    Terminate,
    WindowChange,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::unistd;
    
    #[tokio::test]
    async fn test_signal_handler_creation() {
        let pid = unistd::getpid();
        let handler = SignalHandler::new(pid);
        assert!(handler.is_ok());
    }
}

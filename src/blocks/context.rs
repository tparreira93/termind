// ExecutionContext - Rich context capture for command execution
// This module provides comprehensive context information for each command block

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub environment: EnvironmentContext,
    pub project: Option<ProjectContext>,
    pub git: Option<GitContext>,
    pub system: SystemContext,
    pub filesystem: FileSystemContext,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    pub working_directory: String,
    pub home_directory: Option<String>,
    pub shell: ShellInfo,
    pub key_variables: HashMap<String, String>,
    pub path_entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: ProjectType,
    pub project_root: String,
    pub config_files: Vec<String>,
    pub dependencies: Vec<String>,
    pub virtual_env: Option<String>,
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Docker,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    pub repository_root: String,
    pub current_branch: String,
    pub head_commit: String,
    pub status: GitStatus,
    pub remote_origin: Option<String>,
    pub uncommitted_changes: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub clean: bool,
    pub ahead: i32,
    pub behind: i32,
    pub untracked: i32,
    pub modified: i32,
    pub staged: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub platform: String,
    pub hostname: String,
    pub username: String,
    pub process_id: u32,
    pub parent_process_id: Option<u32>,
    pub cpu_count: usize,
    pub memory_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemContext {
    pub current_files: Vec<FileInfo>,
    pub disk_usage: Option<DiskUsage>,
    pub permissions: FilePermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl ExecutionContext {
    /// Capture full execution context for the current environment
    pub async fn capture() -> Result<Self> {
        let captured_at = Utc::now();
        
        let environment = EnvironmentContext::capture()?;
        let project = ProjectContext::detect(&environment.working_directory).ok();
        let git = GitContext::capture(&environment.working_directory).ok();
        let system = SystemContext::capture()?;
        let filesystem = FileSystemContext::capture(&environment.working_directory)?;

        Ok(Self {
            environment,
            project,
            git,
            system,
            filesystem,
            captured_at,
        })
    }

    /// Get a summary string of the most important context information
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        
        // Working directory (shortened)
        let wd = Path::new(&self.environment.working_directory);
        if let Some(name) = wd.file_name() {
            parts.push(format!("📁 {}", name.to_string_lossy()));
        }

        // Git branch if available
        if let Some(git) = &self.git {
            parts.push(format!("🌿 {}", git.current_branch));
            if git.uncommitted_changes {
                parts.push("🔄".to_string());
            }
        }

        // Project type if detected
        if let Some(project) = &self.project {
            let icon = match project.project_type {
                ProjectType::Rust => "🦀",
                ProjectType::Node => "📦",
                ProjectType::Python => "🐍", 
                ProjectType::Go => "🐹",
                ProjectType::Java => "☕",
                ProjectType::Docker => "🐳",
                ProjectType::Unknown => "❓",
            };
            parts.push(icon.to_string());
        }

        parts.join(" ")
    }
}

impl EnvironmentContext {
    fn capture() -> Result<Self> {
        let working_directory = env::current_dir()
            .map_err(|e| crate::error::TermindError::Io(e))?
            .to_string_lossy()
            .to_string();

        let home_directory = dirs::home_dir().map(|p| p.to_string_lossy().to_string());

        let shell = ShellInfo::detect()?;
        
        // Capture key environment variables
        let key_vars = [
            "USER", "HOME", "PATH", "SHELL", "TERM", "PWD", 
            "LANG", "LC_ALL", "EDITOR", "PAGER"
        ];
        let mut key_variables = HashMap::new();
        for var in key_vars {
            if let Ok(value) = env::var(var) {
                key_variables.insert(var.to_string(), value);
            }
        }

        // Parse PATH entries
        let path_entries = env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(|s| s.to_string())
            .collect();

        Ok(Self {
            working_directory,
            home_directory,
            shell,
            key_variables,
            path_entries,
        })
    }
}

impl ShellInfo {
    fn detect() -> Result<Self> {
        let shell_path = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let shell_name = Path::new(&shell_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sh")
            .to_string();

        // Try to get shell version
        let version = match shell_name.as_str() {
            "zsh" => get_command_output(&[&shell_path, "--version"]),
            "bash" => get_command_output(&[&shell_path, "--version"]),
            "fish" => get_command_output(&[&shell_path, "--version"]),
            _ => None,
        };

        Ok(Self {
            name: shell_name,
            path: shell_path,
            version,
        })
    }
}

impl ProjectContext {
    fn detect(directory: &str) -> Result<Self> {
        let dir_path = Path::new(directory);
        let project_root = Self::find_project_root(dir_path)?;
        
        let (project_type, config_files) = Self::detect_project_type(&project_root);
        let dependencies = Self::extract_dependencies(&project_root, &project_type);
        let virtual_env = Self::detect_virtual_env(&project_root, &project_type);
        let package_manager = Self::detect_package_manager(&project_root, &project_type);

        Ok(Self {
            project_type,
            project_root: project_root.to_string_lossy().to_string(),
            config_files,
            dependencies,
            virtual_env,
            package_manager,
        })
    }

    fn find_project_root(start_dir: &Path) -> Result<PathBuf> {
        let mut current = start_dir;
        
        loop {
            // Check for common project indicators
            let indicators = [
                "Cargo.toml", "package.json", "pyproject.toml", "requirements.txt",
                "go.mod", "pom.xml", "build.gradle", "Dockerfile", ".git"
            ];
            
            for indicator in indicators {
                if current.join(indicator).exists() {
                    return Ok(current.to_path_buf());
                }
            }
            
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                return Ok(start_dir.to_path_buf());
            }
        }
    }

    fn detect_project_type(project_root: &Path) -> (ProjectType, Vec<String>) {
        let mut config_files = Vec::new();
        
        if project_root.join("Cargo.toml").exists() {
            config_files.push("Cargo.toml".to_string());
            return (ProjectType::Rust, config_files);
        }
        
        if project_root.join("package.json").exists() {
            config_files.push("package.json".to_string());
            if project_root.join("yarn.lock").exists() {
                config_files.push("yarn.lock".to_string());
            }
            if project_root.join("package-lock.json").exists() {
                config_files.push("package-lock.json".to_string());
            }
            return (ProjectType::Node, config_files);
        }
        
        if project_root.join("pyproject.toml").exists() || 
           project_root.join("requirements.txt").exists() ||
           project_root.join("setup.py").exists() {
            for file in ["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"] {
                if project_root.join(file).exists() {
                    config_files.push(file.to_string());
                }
            }
            return (ProjectType::Python, config_files);
        }
        
        if project_root.join("go.mod").exists() {
            config_files.push("go.mod".to_string());
            return (ProjectType::Go, config_files);
        }
        
        if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
            for file in ["pom.xml", "build.gradle", "build.gradle.kts"] {
                if project_root.join(file).exists() {
                    config_files.push(file.to_string());
                }
            }
            return (ProjectType::Java, config_files);
        }
        
        if project_root.join("Dockerfile").exists() {
            config_files.push("Dockerfile".to_string());
            if project_root.join("docker-compose.yml").exists() {
                config_files.push("docker-compose.yml".to_string());
            }
            return (ProjectType::Docker, config_files);
        }
        
        (ProjectType::Unknown, config_files)
    }

    fn extract_dependencies(_project_root: &Path, _project_type: &ProjectType) -> Vec<String> {
        // TODO: Parse actual dependencies from config files
        // This would require parsing Cargo.toml, package.json, etc.
        Vec::new()
    }

    fn detect_virtual_env(_project_root: &Path, project_type: &ProjectType) -> Option<String> {
        match project_type {
            ProjectType::Python => {
                // Check for Python virtual environment
                env::var("VIRTUAL_ENV").ok()
                    .or_else(|| env::var("CONDA_DEFAULT_ENV").ok())
            },
            ProjectType::Node => {
                // Check for Node version managers
                env::var("NVM_DIR").ok()
            },
            _ => None,
        }
    }

    fn detect_package_manager(project_root: &Path, project_type: &ProjectType) -> Option<String> {
        match project_type {
            ProjectType::Rust => Some("cargo".to_string()),
            ProjectType::Node => {
                if project_root.join("yarn.lock").exists() {
                    Some("yarn".to_string())
                } else if project_root.join("pnpm-lock.yaml").exists() {
                    Some("pnpm".to_string())
                } else {
                    Some("npm".to_string())
                }
            },
            ProjectType::Python => {
                if project_root.join("Pipfile").exists() {
                    Some("pipenv".to_string())
                } else if project_root.join("pyproject.toml").exists() {
                    Some("poetry".to_string())
                } else {
                    Some("pip".to_string())
                }
            },
            ProjectType::Go => Some("go".to_string()),
            ProjectType::Java => {
                if project_root.join("pom.xml").exists() {
                    Some("maven".to_string())
                } else {
                    Some("gradle".to_string())
                }
            },
            _ => None,
        }
    }
}

impl GitContext {
    fn capture(directory: &str) -> Result<Self> {
        let dir_path = Path::new(directory);
        let repo_root = Self::find_git_root(dir_path)?;
        
        let current_branch = get_command_output(&["git", "-C", &repo_root.to_string_lossy(), "branch", "--show-current"])
            .unwrap_or_else(|| "unknown".to_string());
        
        let head_commit = get_command_output(&["git", "-C", &repo_root.to_string_lossy(), "rev-parse", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        
        let remote_origin = get_command_output(&["git", "-C", &repo_root.to_string_lossy(), "remote", "get-url", "origin"]);
        
        let status = Self::parse_git_status(&repo_root)?;
        
        let staged_files = Self::get_staged_files(&repo_root);
        let modified_files = Self::get_modified_files(&repo_root);
        let uncommitted_changes = !staged_files.is_empty() || !modified_files.is_empty();

        Ok(Self {
            repository_root: repo_root.to_string_lossy().to_string(),
            current_branch,
            head_commit,
            status,
            remote_origin,
            uncommitted_changes,
            staged_files,
            modified_files,
        })
    }

    fn find_git_root(start_dir: &Path) -> Result<PathBuf> {
        let mut current = start_dir;
        
        loop {
            if current.join(".git").exists() {
                return Ok(current.to_path_buf());
            }
            
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                return Err(crate::error::TermindError::Configuration("Not in a git repository".to_string()));
            }
        }
    }

    fn parse_git_status(_repo_root: &Path) -> Result<GitStatus> {
        // This is a simplified implementation
        // In a real implementation, you'd parse `git status --porcelain=v1`
        Ok(GitStatus {
            clean: true,
            ahead: 0,
            behind: 0,
            untracked: 0,
            modified: 0,
            staged: 0,
        })
    }

    fn get_staged_files(_repo_root: &Path) -> Vec<String> {
        // TODO: Implement actual git staged files detection
        Vec::new()
    }

    fn get_modified_files(_repo_root: &Path) -> Vec<String> {
        // TODO: Implement actual git modified files detection
        Vec::new()
    }
}

impl SystemContext {
    fn capture() -> Result<Self> {
        let platform = std::env::consts::OS.to_string();
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        let username = env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        let process_id = std::process::id();
        let parent_process_id = get_parent_process_id();
        let cpu_count = num_cpus::get();
        let memory_total = get_total_memory();

        Ok(Self {
            platform,
            hostname,
            username,
            process_id,
            parent_process_id,
            cpu_count,
            memory_total,
        })
    }
}

impl FileSystemContext {
    fn capture(directory: &str) -> Result<Self> {
        let dir_path = Path::new(directory);
        let mut current_files = Vec::new();

        // Read directory contents (limit to first 50 files)
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for (i, entry) in entries.enumerate() {
                if i >= 50 { break; } // Limit to avoid performance issues
                
                if let Ok(entry) = entry {
                    let metadata = entry.metadata().ok();
                    let file_info = FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        file_type: if metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                            FileType::Directory
                        } else if metadata.as_ref().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                            FileType::Symlink
                        } else {
                            FileType::File
                        },
                        size: metadata.as_ref().and_then(|m| if m.is_file() { Some(m.len()) } else { None }),
                        modified: metadata.as_ref()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0))
                            .flatten(),
                    };
                    current_files.push(file_info);
                }
            }
        }

        let permissions = FilePermissions {
            readable: dir_path.exists() && dir_path.metadata().map(|m| !m.permissions().readonly()).unwrap_or(false),
            writable: is_writable(dir_path),
            executable: is_executable(dir_path),
        };

        let disk_usage = get_disk_usage(dir_path);

        Ok(Self {
            current_files,
            disk_usage,
            permissions,
        })
    }
}

// Helper functions
fn get_command_output(args: &[&str]) -> Option<String> {
    if args.is_empty() {
        return None;
    }
    
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
}

fn get_parent_process_id() -> Option<u32> {
    // This is platform-specific - simplified implementation
    None
}

fn get_total_memory() -> Option<u64> {
    // This would require platform-specific system calls
    None
}

fn is_writable(path: &Path) -> bool {
    // Test if we can write to the directory
    path.metadata()
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

fn is_executable(_path: &Path) -> bool {
    // This is platform-specific
    true
}

fn get_disk_usage(_path: &Path) -> Option<DiskUsage> {
    // This would require platform-specific system calls
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_capture() {
        let context = ExecutionContext::capture().await;
        assert!(context.is_ok());
        
        let ctx = context.unwrap();
        assert!(!ctx.environment.working_directory.is_empty());
        assert!(!ctx.environment.shell.name.is_empty());
        assert!(!ctx.summary().is_empty());
    }

    #[test]
    fn test_shell_detection() {
        let shell = ShellInfo::detect();
        assert!(shell.is_ok());
        
        let shell = shell.unwrap();
        assert!(!shell.name.is_empty());
        assert!(!shell.path.is_empty());
    }

    #[test]
    fn test_project_context() {
        let current_dir = env::current_dir().unwrap();
        let project = ProjectContext::detect(&current_dir.to_string_lossy());
        
        // Should detect this as a Rust project
        if project.is_ok() {
            let proj = project.unwrap();
            assert_eq!(proj.project_type, ProjectType::Rust);
            assert!(proj.config_files.contains(&"Cargo.toml".to_string()));
        }
    }
}

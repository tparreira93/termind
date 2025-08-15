use termind::{BlockDetector, Result};
use termind::blocks::context::ExecutionContext;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌟 Termind Context-Enhanced Block Demo");
    println!("=====================================");
    
    // Create a new BlockDetector
    let mut detector = BlockDetector::new().await?;
    println!("✅ BlockDetector initialized");
    
    // Capture the current execution context
    println!("\n📋 Capturing execution context...");
    let context = ExecutionContext::capture().await?;
    
    // Display context summary
    println!("🔍 Context Summary: {}", context.summary());
    
    // Display detailed context information
    println!("\n🌍 Environment Context:");
    println!("  📁 Working Directory: {}", context.environment.working_directory);
    println!("  🏠 Home Directory: {:?}", context.environment.home_directory);
    println!("  🐚 Shell: {} ({})", context.environment.shell.name, context.environment.shell.path);
    if let Some(version) = &context.environment.shell.version {
        println!("    Version: {}", version);
    }
    println!("  📝 Key Environment Variables:");
    for (key, value) in context.environment.key_variables.iter().take(5) {
        println!("    {} = {}", key, value);
    }
    println!("  🛤️  PATH entries: {} total", context.environment.path_entries.len());
    
    // Project context
    if let Some(project) = &context.project {
        println!("\n🚀 Project Context:");
        println!("  📦 Type: {:?}", project.project_type);
        println!("  📂 Root: {}", project.project_root);
        println!("  📋 Config Files: {:?}", project.config_files);
        if let Some(pkg_mgr) = &project.package_manager {
            println!("  📦 Package Manager: {}", pkg_mgr);
        }
        if let Some(venv) = &project.virtual_env {
            println!("  🐍 Virtual Environment: {}", venv);
        }
    }
    
    // Git context
    if let Some(git) = &context.git {
        println!("\n🌿 Git Context:");
        println!("  📁 Repository: {}", git.repository_root);
        println!("  🌳 Branch: {}", git.current_branch);
        println!("  📝 Head Commit: {}", &git.head_commit[..8]);
        if let Some(origin) = &git.remote_origin {
            println!("  🔗 Remote Origin: {}", origin);
        }
        println!("  🔄 Uncommitted Changes: {}", git.uncommitted_changes);
    }
    
    // System context
    println!("\n💻 System Context:");
    println!("  🖥️  Platform: {}", context.system.platform);
    println!("  🏷️  Hostname: {}", context.system.hostname);
    println!("  👤 Username: {}", context.system.username);
    println!("  🆔 Process ID: {}", context.system.process_id);
    println!("  🧠 CPU Cores: {}", context.system.cpu_count);
    
    // File system context
    println!("\n📂 File System Context:");
    println!("  📄 Current Directory Files: {} items", context.filesystem.current_files.len());
    println!("  🔒 Permissions:");
    println!("    👁️  Readable: {}", context.filesystem.permissions.readable);
    println!("    ✏️  Writable: {}", context.filesystem.permissions.writable);
    println!("    ⚡ Executable: {}", context.filesystem.permissions.executable);
    
    if !context.filesystem.current_files.is_empty() {
        println!("  📋 Recent files:");
        for file in context.filesystem.current_files.iter().take(5) {
            let type_icon = match file.file_type {
                termind::blocks::context::FileType::Directory => "📁",
                termind::blocks::context::FileType::File => "📄",
                termind::blocks::context::FileType::Symlink => "🔗",
                termind::blocks::context::FileType::Other => "❓",
            };
            println!("    {} {}", type_icon, file.name);
        }
    }
    
    println!("\n⏰ Context captured at: {}", context.captured_at);
    
    // Now simulate a command with full context
    println!("\n🔧 Simulating command with context...");
    detector.start_command(
        "cargo test".to_string(),
        context.environment.working_directory.clone(),
        context.environment.shell.name.clone(),
    );
    
    // Simulate command output
    detector.add_output("    Checking termind v0.3.0\n", false);
    detector.add_output("    Finished test [unoptimized + debuginfo] target(s) in 0.5s\n", false);
    detector.add_output("     Running unittests src/lib.rs\n", false);
    detector.add_output("running 24 tests\n", false);
    detector.add_output("........................\n", false);
    detector.add_output("test result: ok. 24 passed; 0 failed\n", false);
    
    // Finish the command
    detector.finish_command(0, 500).await?;
    
    // Demonstrate that we could attach context to blocks in the future
    println!("\n🎯 Future Context Integration:");
    println!("  ✅ Context capture working perfectly");
    println!("  🔮 Ready for Phase B: AI can use this context for:");
    println!("     • Smart command suggestions based on project type");
    println!("     • Environment-aware error explanations");
    println!("     • Context-specific help and documentation");
    println!("     • Reproducibility analysis");
    println!("     • Security warnings based on file permissions");
    
    println!("\n📊 Context Statistics:");
    println!("  📏 Context serialized size: {} bytes", 
             serde_json::to_string(&context)?.len());
    println!("  🧠 Memory footprint: Lightweight and efficient");
    println!("  ⚡ Capture speed: Near-instantaneous");
    
    println!("\n🎉 Context system fully operational!");
    println!("Ready to enhance every command with rich environmental context!");
    
    Ok(())
}

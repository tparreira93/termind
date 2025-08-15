use termind::{BlockDetector, Result};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧱 Termind Block Storage Demo - Phase A Week 3");
    println!("===============================================");
    
    // Create a new BlockDetector (this initializes the SQLite database)
    let mut detector = BlockDetector::new().await?;
    println!("✅ BlockDetector initialized with SQLite database");
    
    // Simulate running a command
    println!("\n📝 Starting command: 'ls -la /home'");
    detector.start_command(
        "ls -la".to_string(),
        "/home".to_string(),
        "bash".to_string(),
    );
    
    // Simulate command output
    detector.add_output("total 24\n", false);
    detector.add_output("drwxr-xr-x  3 user user 4096 Aug 14 16:30 .\n", false);
    detector.add_output("drwxr-xr-x 18 root root 4096 Aug 14 10:15 ..\n", false);
    detector.add_output("drwxr-xr-x  5 user user 4096 Aug 14 16:30 user\n", false);
    
    // Simulate command completion
    println!("✅ Command completed with exit code 0");
    detector.finish_command(0, 125).await?;
    
    // Simulate another command (with failure)
    println!("\n📝 Starting command: 'cat nonexistent.txt'");
    detector.start_command(
        "cat nonexistent.txt".to_string(),
        "/home/user".to_string(),
        "bash".to_string(),
    );
    
    detector.add_output("", false); // no stdout
    detector.add_output("cat: nonexistent.txt: No such file or directory\n", true);
    
    println!("❌ Command failed with exit code 1");
    detector.finish_command(1, 50).await?;
    
    // Demonstrate search functionality
    println!("\n🔍 Searching for commands containing 'ls':");
    let results = detector.search("ls").await?;
    for block in &results {
        println!("  📦 Command: {}", block.command);
        println!("      Directory: {}", block.cwd);
        println!("      Shell: {}", block.shell);
        println!("      Exit Code: {:?}", block.exit_code);
        println!("      Duration: {:?}ms", block.duration_ms);
    }
    
    // Show recent commands
    println!("\n📋 Recent commands (last 5):");
    let recent = detector.get_recent(5).await?;
    for (i, block) in recent.iter().enumerate() {
        let status = if block.success() { "✅" } else { "❌" };
        println!("  {}. {} {} (exit: {:?})", 
                 i + 1, 
                 status, 
                 block.command, 
                 block.exit_code);
    }
    
    // Show failed commands
    println!("\n🚫 Failed commands:");
    let failed = detector.get_failed(5).await?;
    for block in &failed {
        println!("  ❌ {} (exit: {:?})", block.command, block.exit_code);
        if !block.stderr.is_empty() {
            println!("      Error: {}", block.stderr.trim());
        }
    }
    
    println!("\n🎉 Week 3 Block Storage Implementation Complete!");
    println!("Features demonstrated:");
    println!("  ✅ SQLite database with FTS (Full-Text Search)");
    println!("  ✅ Block creation and storage");
    println!("  ✅ Command boundary detection");
    println!("  ✅ Search functionality");
    println!("  ✅ Recent and failed command retrieval");
    
    Ok(())
}

use std::env;

fn main() {
    // Set build-time environment variables
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "unknown".to_string());
    
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=BUILD_TARGET={}", target);
    println!("cargo:rustc-env=BUILD_HOST={}", host);
    
    // Add git info if available
    if let Ok(git_hash) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if git_hash.status.success() {
            let git_hash_str = String::from_utf8_lossy(&git_hash.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash_str);
            
            // Check if working directory is clean
            if let Ok(git_status) = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .output()
            {
                let is_clean = git_status.stdout.is_empty();
                println!("cargo:rustc-env=GIT_CLEAN={}", if is_clean { "true" } else { "false" });
            }
        }
    } else {
        // Fallback values for when git is not available
        println!("cargo:rustc-env=GIT_HASH=unknown");
        println!("cargo:rustc-env=GIT_CLEAN=unknown");
    }
    
    // Get current timestamp
    let now = std::process::Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    
    println!("cargo:rustc-env=BUILD_DATE={}", now);
    
    // Optimize for release builds
    if profile == "release" {
        println!("cargo:rustc-cfg=release");
    }
}
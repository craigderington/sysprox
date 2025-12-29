// Version and build information

use std::env;

/// Build information structure
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub version: String,
    pub build_date: String,
    pub build_hash: String,
    pub target_triple: String,
    pub rust_version: String,
    pub optimized: bool,
    pub features: String,
}

/// Get the current version from environment or default
pub fn version() -> String {
    env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string())
}

/// Format version for header display
pub fn format_header() -> String {
    let build = build_info();
    format!("Sysprox v{} - Systemd Service Monitor", build.version)
}

/// Format version for footer display
pub fn format_footer() -> String {
    let build = build_info();
    format!("v{} [{}]",
            build.version,
            if build.optimized { "Release" } else { "Debug" })
}

/// Get current build information
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: version(),
        build_date: option_env!("BUILD_DATE")
            .or_else(|| option_env!("CARGO_BUILD_DATE"))
            .unwrap_or("unknown")
            .to_string(),
        build_hash: option_env!("GIT_HASH").unwrap_or("unknown").to_string(),
        target_triple: option_env!("TARGET").unwrap_or("unknown").to_string(),
        rust_version: option_env!("RUSTC_VERSION").unwrap_or("unknown").to_string(),
        optimized: cfg!(not(debug_assertions)),
        features: option_env!("CARGO_FEATURES").unwrap_or("default").to_string(),
    }
}

/// Version display format
impl BuildInfo {
    pub fn format_display(&self) -> String {
        format!("sysprox v{}", self.version)
    }
    
    pub fn format_detailed(&self) -> String {
        let mut result = format!("sysprox v{}", self.version);
        
        if !self.build_hash.is_empty() && self.build_hash != "unknown" {
            result.push_str(&format!(" (commit {})", self.build_hash));
        }
        
        if let Ok(clean) = env::var("GIT_CLEAN") {
            if clean == "false" {
                result.push_str(" [dirty]");
            }
        }
        
        result
    }
    
    pub fn format_build_info(&self) -> String {
        format!(
            "Build: {}\nTarget: {}\nProfile: {}\nGit: {}\nClean: {}",
            self.build_date,
            self.target_triple,
            if self.optimized { "release" } else { "debug" },
            self.build_hash,
            option_env!("GIT_CLEAN").unwrap_or("unknown")
        )
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            build_date: "unknown".to_string(),
            build_hash: "unknown".to_string(),
            target_triple: "unknown".to_string(),
            rust_version: "unknown".to_string(),
            optimized: false,
            features: "default".to_string(),
        }
    }
}
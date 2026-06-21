use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if assets or project config files change
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=../CrawlCipher.Core/CrawlCipher.Core.csproj");
    println!("cargo:rerun-if-changed=../smart-contracts/session-lock/Cargo.toml");

    // Default fallbacks
    let mut core_version = "0.2.0".to_string();
    let mut contract_version = "0.2.0".to_string();

    // Parse Core version from CrawlCipher.Core.csproj
    let csproj_path = Path::new("../CrawlCipher.Core/CrawlCipher.Core.csproj");
    if csproj_path.exists() {
        if let Ok(content) = fs::read_to_string(csproj_path) {
            if let Some(start) = content.find("<Version>") {
                if let Some(end) = content.find("</Version>") {
                    let version = content[start + 9..end].trim();
                    if !version.is_empty() {
                        core_version = version.to_string();
                    }
                }
            }
        }
    }

    // Parse Contract version from smart-contracts/session-lock/Cargo.toml
    let contract_path = Path::new("../smart-contracts/session-lock/Cargo.toml");
    if contract_path.exists() {
        if let Ok(content) = fs::read_to_string(contract_path) {
            let mut in_package = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "[package]" {
                    in_package = true;
                } else if trimmed.starts_with('[') {
                    in_package = false;
                }
                if in_package && trimmed.starts_with("version") {
                    if let Some(eq_idx) = trimmed.find('=') {
                        let version = trimmed[eq_idx + 1..].trim().trim_matches('"');
                        if !version.is_empty() {
                            contract_version = version.to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    // Export environment variables to be read by env! macro at compile time
    println!("cargo:rustc-env=CORE_VERSION={}", core_version);
    println!("cargo:rustc-env=CONTRACT_VERSION={}", contract_version);
}

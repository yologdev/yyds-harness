fn main() {
    // Expose git short hash at compile time
    if std::env::var("GIT_HASH").is_err() {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
        {
            if output.status.success() {
                let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-env=GIT_HASH={hash}");
            }
        }
    }

    // Expose build date at compile time if not already set
    if std::env::var("BUILD_DATE").is_err() {
        // Use a simple date from the build environment
        if let Ok(output) = std::process::Command::new("date")
            .args(["+%Y-%m-%d"])
            .output()
        {
            if output.status.success() {
                let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-env=BUILD_DATE={date}");
            }
        }
    }

    // Expose evolution day count at compile time (only present in yoyo's own repo)
    if std::env::var("DAY_COUNT").is_err() {
        if let Ok(content) = std::fs::read_to_string("DAY_COUNT") {
            if let Ok(day) = content.trim().parse::<u32>() {
                println!("cargo:rustc-env=DAY_COUNT={day}");
            }
        }
    }
    println!("cargo:rerun-if-changed=DAY_COUNT");

    // Read yoagent version from Cargo.lock (more reliable than parsing Cargo.toml)
    if let Ok(lock_content) = std::fs::read_to_string("Cargo.lock") {
        for chunk in lock_content.split("\n[[package]]") {
            let mut name = None;
            let mut version = None;
            for line in chunk.lines() {
                let line = line.trim();
                if let Some(n) = line.strip_prefix("name = \"") {
                    name = n.strip_suffix('"');
                }
                if let Some(v) = line.strip_prefix("version = \"") {
                    version = v.strip_suffix('"');
                }
            }
            if name == Some("yoagent") {
                if let Some(v) = version {
                    println!("cargo:rustc-env=YOAGENT_VERSION={v}");
                }
                break;
            }
        }
    }
}

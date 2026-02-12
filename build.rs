use std::process::Command;

fn main() {
    // Try pkg-config first
    if let Ok(output) = Command::new("pkg-config")
        .args(["--libs-only-L", "mpv"])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            for flag in path.split_whitespace() {
                if let Some(dir) = flag.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={dir}");
                }
            }
            return;
        }
    }

    // Fallback: try homebrew on macOS
    if cfg!(target_os = "macos") {
        if let Ok(output) = Command::new("brew").args(["--prefix", "mpv"]).output() {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-link-search=native={prefix}/lib");
                return;
            }
        }
    }

    // If neither works, let the linker try its default paths
    eprintln!("Warning: Could not find mpv via pkg-config or homebrew. Make sure libmpv is installed.");
}

use std::path::Path;
use std::process::Command;

fn main() {
    let web_src = Path::new("web/src");
    let web_dist = Path::new("web/dist");

    if !web_src.exists() {
        eprintln!("web/src does not exist, skipping frontend build");
        return;
    }

    // Check if we need to build
    let needs_build = !web_dist.exists()
        || std::fs::read_dir(web_src)
            .map(|mut dir| {
                dir.any(|e| {
                    e.as_ref()
                        .ok()
                        .and_then(|e| e.metadata().ok())
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            web_dist
                                .metadata()
                                .ok()
                                .and_then(|m| m.modified().ok())
                                .map(|dt| t > dt)
                                .unwrap_or(true)
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

    if !needs_build {
        eprintln!("web/dist exists and is up to date, skipping frontend build");
        return;
    }

    eprintln!("Building web frontend...");

    // npm install
    let output = Command::new("npm")
        .args(["install"])
        .current_dir("web")
        .output()
        .expect("Failed to execute npm install");

    if !output.status.success() {
        eprintln!("npm install failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    // npm run build
    let output = Command::new("npm")
        .args(["run", "build"])
        .current_dir("web")
        .output()
        .expect("Failed to execute npm run build");

    if !output.status.success() {
        eprintln!("npm run build failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    eprintln!("Frontend build complete.");

    // Tell cargo to rerun if web/src changes
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/vite.config.js");
}

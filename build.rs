use std::{fs, path::Path, process::Command, time::SystemTime};

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    if let Some(commit_id) = git_commit_id() {
        println!("cargo:rustc-env=BABATA_BUILD_COMMIT={commit_id}");
    }

    build_web_ui();
}

/// Build the Web UI if `web/dist` is missing or older than source files.
///
/// This ensures that `cargo build` always produces a binary whose bundled
/// static files match the latest frontend source, preventing the stale-dist
/// bug where PRs fix `web/src` but the runtime continues to serve an old
/// `web/dist` that was never rebuilt.
fn build_web_ui() {
    let web_dir = Path::new("web");
    let dist_index = web_dir.join("dist/index.html");

    // Input paths that should trigger a rebuild when they change.
    // We watch these directories/files so cargo reruns this script
    // whenever the frontend source is modified.
    // We intentionally do NOT watch `web/dist` to avoid build loops.
    let watched_inputs: &[&str] = &[
        "src",
        "package.json",
        "package-lock.json",
        "vite.config.ts",
        "index.html",
        "public",
        "components.json",
        "tsconfig.json",
        "tsconfig.app.json",
        "tsconfig.node.json",
    ];

    for input in watched_inputs {
        println!("cargo:rerun-if-changed=web/{input}");
    }

    // Skip build if dist/index.html exists and is at least as new as every
    // watched source file.
    if dist_index.exists() {
        let dist_mtime = mtime(&dist_index).unwrap_or(SystemTime::UNIX_EPOCH);
        if let Some(newest_src) = newest_mtime_among(web_dir, watched_inputs)
            && dist_mtime >= newest_src
        {
            return;
        }
    }

    let npm = npm_cmd();

    let node_modules_lock = web_dir.join("node_modules/.package-lock.json");
    let package_lock = web_dir.join("package-lock.json");

    let needs_npm_install = !node_modules_lock.exists()
        || mtime(&package_lock).unwrap_or(SystemTime::UNIX_EPOCH)
            > mtime(&node_modules_lock).unwrap_or(SystemTime::UNIX_EPOCH);

    if needs_npm_install {
        println!("cargo:warning=Running npm install in web/ ...");
        run_cmd(
            npm,
            &["install"],
            web_dir,
            "npm install failed. \
             Please ensure Node.js and npm are installed and available in PATH.",
        );
    }

    println!("cargo:warning=Running npm run build in web/ ...");
    run_cmd(
        npm,
        &["run", "build"],
        web_dir,
        "npm run build failed. \
         Please ensure Node.js and npm are installed and available in PATH.",
    );

    if !dist_index.exists() {
        panic!(
            "Web UI build completed but web/dist/index.html was not produced. \
             Check the build output above for errors."
        );
    }
}

/// Return the npm executable name for the current platform.
fn npm_cmd() -> &'static str {
    if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    }
}

/// Run a command, failing with a descriptive message on error.
fn run_cmd(cmd: &str, args: &[&str], cwd: &Path, context: &str) {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "{context}\n\
                 Failed to spawn '{cmd}': {e}"
            );
        });

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "{context}\n\
             Command: {cmd} {}\n\
             Exit code: {}\n\
             stdout:\n{stdout}\n\
             stderr:\n{stderr}",
            args.join(" "),
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_string(), |c| c.to_string())
        );
    }
}

/// Get the modification time of a path, if available.
fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// Find the newest modification time among a set of paths relative to `base`.
fn newest_mtime_among(base: &Path, inputs: &[&str]) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    for input in inputs {
        let path = base.join(input);
        if !path.exists() {
            continue;
        }
        let candidate = if path.is_dir() {
            newest_mtime_in_dir(&path)
        } else {
            mtime(&path)
        };
        if let Some(candidate) = candidate {
            newest = Some(newest.map_or(candidate, |n| n.max(candidate)));
        }
    }
    newest
}

/// Recursively find the newest modification time in a directory.
fn newest_mtime_in_dir(dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let candidate = if path.is_dir() {
            newest_mtime_in_dir(&path)
        } else {
            mtime(&path)
        };
        if let Some(candidate) = candidate {
            newest = Some(newest.map_or(candidate, |n| n.max(candidate)));
        }
    }
    newest
}

fn git_commit_id() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let commit_id = String::from_utf8(output.stdout).ok()?;
    let commit_id = commit_id.trim();
    if commit_id.is_empty() {
        return None;
    }

    Some(commit_id.to_string())
}

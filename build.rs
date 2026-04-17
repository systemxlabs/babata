use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    if let Some(commit_id) = git_commit_id() {
        println!("cargo:rustc-env=BABATA_BUILD_COMMIT={commit_id}");
    }
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

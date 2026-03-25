use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/index.html");

    let status = Command::new("npm")
        .args(["--prefix", "web", "run", "build"])
        .status()
        .expect("failed to execute `npm --prefix web run build`");

    if !status.success() {
        panic!("`npm --prefix web run build` failed with status: {status}");
    }
}

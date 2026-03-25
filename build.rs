use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");
    println!("cargo:rerun-if-changed=web/tsconfig.json");
    println!("cargo:rerun-if-changed=web/vite.config.ts");

    let status = Command::new("npm")
        .args(["--prefix", "web", "run", "build"])
        .status()
        .expect("failed to execute `npm --prefix web run build`");

    if !status.success() {
        panic!("`npm --prefix web run build` failed with status: {status}");
    }
}

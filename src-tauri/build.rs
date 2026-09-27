fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");
    let revision = std::env::var("GITHUB_SHA").ok().or_else(|| {
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
    });
    if let Some(revision) = revision {
        let revision = revision.trim();
        if revision.len() >= 7
            && revision.len() <= 64
            && revision.chars().all(|c| c.is_ascii_hexdigit())
        {
            println!("cargo:rustc-env=DMSITE_BUILD_REVISION={revision}");
        }
    }
    tauri_build::build()
}

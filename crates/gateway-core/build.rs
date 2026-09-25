use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
    println!("cargo:rerun-if-env-changed=BUILD_TIME");

    let sha = git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let short_sha =
        git_output(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let tag = git_output(&["describe", "--tags", "--exact-match"]).unwrap_or_default();
    let dirty = git_output(&["status", "--porcelain"]).is_some();

    println!("cargo:rustc-env=GATEWAY_GIT_SHA={sha}");
    println!("cargo:rustc-env=GATEWAY_GIT_SHORT_SHA={short_sha}");
    println!("cargo:rustc-env=GATEWAY_GIT_TAG={tag}");
    println!("cargo:rustc-env=GATEWAY_GIT_DIRTY={dirty}");

    let build_time = std::env::var("BUILD_TIME").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=GATEWAY_BUILD_TIME={build_time}");
}

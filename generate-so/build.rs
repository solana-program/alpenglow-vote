use std::{env, fs, path::PathBuf};

fn main() {
    // Get project directory
    let base_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();

    // src: project-dir/target/deploy/alpenglow_vote.so
    let deploy_so_path = base_path
        .join("target")
        .join("deploy")
        .join("alpenglow_vote.so");

    // dest: project-dir/spl-alpenglow_vote.so
    let dest_path = base_path.join("spl-alpenglow_vote.so");

    // Save the destination path as an environment variable that can later be invoked in Rust code
    println!(
        "cargo:rustc-env=ALPENGLOW_VOTE_SO_PATH={}",
        dest_path.display()
    );

    // copy from src to dest
    fs::copy(&deploy_so_path, &dest_path).unwrap_or_else(|_| {
        panic!(
            "Couldn't copy spl-alpenglow_vote.so from {} to {}.",
            &deploy_so_path.display(),
            &dest_path.display()
        )
    });
}

use std::{env, fs, path::PathBuf};

fn main() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let base_path = PathBuf::from(cargo_manifest_dir)
        .parent()
        .unwrap()
        .to_path_buf();

    let deploy_so_path = base_path
        .join("target")
        .join("deploy")
        .join("alpenglow_vote.so");

    let dest_path = base_path.join("spl-alpenglow_vote.so");

    fs::copy(deploy_so_path, dest_path).expect("Couldn't copy spl-alpenglow_vote.so.");
}

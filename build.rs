use std::{env, process::Command};

fn main() {
    // Tell Cargo that if the frontend changes, to rerun this build script.
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/static");
    println!("cargo:rerun-if-changed=frontend/package.json");

    // specific weirdness on windows, need workaround
    #[cfg(windows)]
    pub const NPM: &'static str = "npm.cmd";
    #[cfg(not(windows))]
    pub const NPM: &'static str = "npm";

    // Can't pass --prefix to npm.cmd for some reason, so cd to where the frontend's based
    env::set_current_dir("frontend").unwrap();
    Command::new(NPM).args(["install"]).status().unwrap();
    Command::new(NPM).args(["run", "build"]).status().unwrap();

    // Rerun on sqlx migrations changing
    println!("cargo:rerun-if-changed=migrations");
}

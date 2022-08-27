use std::env;

fn main() {
    let target_triple = env::var("TARGET").unwrap();
    println!(
        "cargo:rustc-env=LINTJE_BUILD_TARGET_TRIPLE={}",
        target_triple
    );
}

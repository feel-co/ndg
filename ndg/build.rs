use std::env;

fn main() {
  // XXX: This cannot be done in the Cargo configuration, so we had to
  // create a build wrapper to make it work. This ensures that we can
  // append our own C compile flags to try and optimize the *numerous*
  // Treesitter dependencies that get pulled for syntax highlighting.
  if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
    // Preserve existing CFLAGS if present
    let mut cflags = env::var("CFLAGS").unwrap_or_default();

    if !cflags.is_empty() {
      cflags.push(' ');
    }

    cflags.push_str("-O2 -ffunction-sections -fdata-sections");

    // Export for cc crate and build scripts
    println!("cargo:rustc-env=CFLAGS={cflags}");
    println!("cargo:rustc-env=CXXFLAGS={cflags}");

    // Potentially optimize TS grammars by ensuring that the linker can discard
    // and fold grammar sections. Though, this is not a silver bullet. TS
    // grammars are mainly data heavy, which we cannot optimize :(
    println!("cargo:rustc-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-link-arg=-Wl,--icf=all");

    // Improve static archive behavior
    println!("cargo:rustc-env=ARFLAGS=crs");
  }
}

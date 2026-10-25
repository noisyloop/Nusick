//! # xtask
//!
//! Workspace build helper, run as `cargo xtask <command>`. It wraps nih-plug's
//! bundler, which builds the plugin and lays out the `.clap` (and, with
//! `--features vst3-gpl`, `.vst3`) bundles in `target/bundled/`.
//!
//! ```text
//! cargo xtask bundle kick_me --release
//! cargo xtask bundle kick_me --release --features vst3-gpl   # GPLv3 build!
//! ```

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()
}

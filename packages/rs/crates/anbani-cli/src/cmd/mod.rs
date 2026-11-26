//! One module per command group.

pub mod art;
pub mod core;
pub mod data;
pub mod inspect;
pub mod legacy;
pub mod lorem;
pub mod mcp;
pub mod morph;
pub mod numbers;
pub mod phon;
pub mod spell;
pub mod stats;
pub mod text;
pub mod tui;

/// A command that takes only text.
#[derive(clap::Args)]
pub struct TextArg {
    /// Text; `-` or nothing reads stdin
    pub text: Option<String>,
}

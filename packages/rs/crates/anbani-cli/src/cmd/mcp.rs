//! `anbani mcp`: the MCP server on stdio (crate anbani-mcp). Register it with an MCP client as the
//! command `anbani mcp`.

use std::io;

use crate::io::Out;

pub fn run() -> Result<Out, Box<dyn std::error::Error>> {
    anbani_mcp::serve(io::stdin().lock(), io::stdout().lock())?;
    Ok(Out::Silent)
}

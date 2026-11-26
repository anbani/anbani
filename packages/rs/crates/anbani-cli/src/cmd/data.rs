//! `anbani data`: install and inspect data packs. Packs are pinned by SHA-256 in this build;
//! downloads go through `curl` (no TLS stack in the static binary). Private releases are read with `$GH_TOKEN` /
//! `$GITHUB_TOKEN`, else `gh auth token`; the token goes to curl on stdin, never on its command line.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anbani_spell::pack::{self, Pinned};
use serde_json::{json, Value};

use crate::io::Out;

/// `anbani data`.
#[derive(clap::Args)]
pub struct DataArgs {
    #[command(subcommand)]
    command: DataCommand,
}

#[derive(clap::Subcommand)]
enum DataCommand {
    /// Download a pack, check every file against its pinned SHA-256, install it atomically
    Fetch {
        /// The pack: spell
        name: String,
        /// Install from a local directory or a base URL instead of the pinned GitHub release
        #[arg(long)]
        from: Option<String>,
    },
    /// Installed packs and whether their files match the pins
    List,
    /// The data directory
    Path,
}

fn pinned(name: &str) -> Result<Pinned, Box<dyn std::error::Error>> {
    match name {
        "spell" => Ok(pack::SPELL),
        other => Err(format!("unknown pack '{other}' (known: spell)").into()),
    }
}

pub fn run(a: &DataArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let root = pack::data_root().ok_or("no data directory: set ANBANI_DATA or HOME")?;
    match &a.command {
        DataCommand::Path => Ok(Out::Text(root.display().to_string())),
        DataCommand::List => {
            let dir = root.join("spell");
            let state = if !dir.join("spellcheck.meta.json").exists() {
                "missing"
            } else if pack::SPELL.files.iter().all(|(f, sha)| pack::verify_file(&dir, f, sha).is_ok()) {
                "pinned"
            } else {
                "unpinned"
            };
            Ok(Out::Json(json!([{"name": "spell", "tag": pack::SPELL.tag, "dir": dir.display().to_string(),
                "state": state}])))
        }
        DataCommand::Fetch { name, from } => {
            let pin = pinned(name)?;
            let dest = root.join(name);
            fetch(&pin, from.as_deref(), &dest)?;
            Ok(Out::Text(format!("{name} {} → {}", pin.tag, dest.display())))
        }
    }
}

fn fetch(pin: &Pinned, from: Option<&str>, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let partial = dest.with_extension("partial");
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    fs::create_dir_all(&partial)?;
    let assets = match from {
        None => Some(release_assets(pin)?),
        Some(_) => None,
    };
    for (file, sha) in pin.files {
        let bytes = match (from, &assets) {
            (Some(src), _) if !src.starts_with("http") => fs::read(Path::new(src).join(file))?,
            (Some(base), _) => curl(&format!("{}/{file}", base.trim_end_matches('/')), None, false)?,
            (None, Some(Source::Api { token, assets })) => {
                let url = assets.get(*file).ok_or_else(|| format!("release {} has no {file}", pin.tag))?;
                curl(url, Some(token), true)?
            }
            (None, _) => {
                let url = format!("https://github.com/{}/releases/download/{}/{file}", pin.repo, pin.tag);
                curl(&url, None, false)?
            }
        };
        let actual = pack::sha256_hex(&bytes);
        if actual != *sha {
            return Err(format!("{file}: sha256 {actual}, pinned {sha}; nothing installed").into());
        }
        fs::write(partial.join(file), bytes)?;
    }
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::rename(&partial, dest)?;
    Ok(())
}

enum Source {
    Public,
    Api { token: String, assets: std::collections::HashMap<String, String> },
}

fn token() -> Option<String> {
    let env = ["GH_TOKEN", "GITHUB_TOKEN"].iter().find_map(|k| std::env::var(k).ok().filter(|t| !t.is_empty()));
    env.or_else(|| {
        let out = Command::new("gh").args(["auth", "token"]).stderr(Stdio::null()).output().ok()?;
        let t = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && !t.is_empty()).then_some(t)
    })
}

/// With a token, the release's asset API urls (works for private repos); without one, public download urls.
fn release_assets(pin: &Pinned) -> Result<Source, Box<dyn std::error::Error>> {
    let Some(token) = token() else {
        return Ok(Source::Public);
    };
    let url = format!("https://api.github.com/repos/{}/releases/tags/{}", pin.repo, pin.tag);
    let release: Value = serde_json::from_slice(&curl(&url, Some(&token), false)?)?;
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| format!("release {} lists no assets", pin.tag))?
        .iter()
        .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["url"].as_str()?.to_string())))
        .collect();
    Ok(Source::Api { token, assets })
}

fn curl(url: &str, token: Option<&str>, octet: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--retry", "2", "-H", "User-Agent: anbani.rs"]);
    if octet {
        cmd.args(["-H", "Accept: application/octet-stream"]);
    }
    if token.is_some() {
        cmd.args(["-H", "@-"]);
    }
    cmd.arg(url).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("curl is needed for downloads: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Some(t) = token {
            writeln!(stdin, "Authorization: Bearer {t}")?;
        }
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(format!("download failed: {url}: {}", String::from_utf8_lossy(&out.stderr).trim()).into());
    }
    Ok(out.stdout)
}

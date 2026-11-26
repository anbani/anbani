//! Spell packs on disk: where they live, loading one, and checking files against pinned SHA-256 hashes.
//! A pack is a directory with `spellcheck.bin`, `spellcheck.meta.json`, optionally
//! `spellcheck.freq.txt`, `SOURCES.md` and `SHA256SUMS`, as Anbani.Data.Spellcheck publishes it.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{Dawg, Error, Speller};

/// A release of the licensed pack this build trusts: every file with its SHA-256.
#[derive(Debug, Clone, Copy)]
pub struct Pinned {
    /// GitHub `owner/repo` that publishes it.
    pub repo: &'static str,
    /// The release tag.
    pub tag: &'static str,
    /// `(file name, sha256 hex)`.
    pub files: &'static [(&'static str, &'static str)],
}

/// The pack `anbani data fetch spell` installs (Anbani.Data.Spellcheck `make licensed`: Hunspell `ka_GE` + the
/// Anbani wordlist): 484 792 words, 124 457 states, 2.0 MB.
pub const SPELL: Pinned = Pinned {
    repo: "OdyssevsCom/Anbani.Data.Spellcheck",
    tag: "spell-ka-licensed-30c0630d0bb5",
    files: &[
        ("spellcheck.bin", "30c0630d0bb50a1d065a873711dab9efb8f7584b624f50b73fe1959a4203f8dc"),
        ("spellcheck.meta.json", "23a93be797b4264a629df2b432b8ac1f35adbf72718aec465d543036ad5706c9"),
        ("spellcheck.freq.txt", "3d8d2b2e20fdd22be72eda67ecf65ab3c809038a2ab2c3d52127577aa42b7305"),
        ("SOURCES.md", "79c0f12907cba0128c014bab402741d34aa97325010f1753304625e6a3598a3c"),
    ],
};

/// The data root: `$ANBANI_DATA`, else `$XDG_DATA_HOME/anbani`, else `$HOME/.local/share/anbani`.
pub fn data_root() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    var("ANBANI_DATA")
        .or_else(|| var("XDG_DATA_HOME").map(|d| d.join("anbani")))
        .or_else(|| var("HOME").map(|h| h.join(".local/share/anbani")))
}

/// Where the spell pack is installed: `<data root>/spell`.
pub fn spell_dir() -> Option<PathBuf> {
    data_root().map(|r| r.join("spell"))
}

/// Lower-case hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes).iter().fold(String::with_capacity(64), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    })
}

fn read(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))
}

/// Check `dir/<file>` against `expected` hex.
///
/// # Errors
/// `Io` when unreadable, `Checksum` when it differs.
pub fn verify_file(dir: &Path, file: &str, expected: &str) -> Result<(), Error> {
    let actual = sha256_hex(&read(&dir.join(file))?);
    if actual == expected {
        Ok(())
    } else {
        Err(Error::Checksum { file: file.to_string(), expected: expected.to_string(), actual })
    }
}

/// Check every file a `SHA256SUMS` in `dir` lists (`<hex>  <name>` lines); returns the names checked.
///
/// # Errors
/// `Io` when `SHA256SUMS` or a file is missing, `Checksum` on a mismatch, `Meta` for a malformed line.
pub fn verify_sums(dir: &Path) -> Result<Vec<String>, Error> {
    let sums = String::from_utf8_lossy(&read(&dir.join("SHA256SUMS"))?).into_owned();
    let mut names = Vec::new();
    for line in sums.lines().filter(|l| !l.trim().is_empty()) {
        let Some((hex, name)) = line.split_once(char::is_whitespace) else {
            return Err(Error::Meta(format!("SHA256SUMS line '{line}'")));
        };
        let name = name.trim_start().trim_start_matches('*');
        verify_file(dir, name, hex)?;
        names.push(name.to_string());
    }
    Ok(names)
}

/// Load the pack in `dir` into a [`Speller`] (frequency table attached when the meta names one that exists).
///
/// # Errors
/// `Io` for a missing file, `Meta` for a malformed meta, and every [`Dawg::from_bytes`] error.
pub fn load(dir: &Path) -> Result<Speller, Error> {
    let meta_bytes = read(&dir.join("spellcheck.meta.json")).map_err(|_| {
        Error::Io(format!(
            "no spell pack in {} (spellcheck.meta.json missing): run `anbani data fetch spell` or pass --pack DIR",
            dir.display()
        ))
    })?;
    let meta: serde_json::Value = serde_json::from_slice(&meta_bytes).map_err(|e| Error::Meta(e.to_string()))?;
    let alphabet = meta["alphabet"]
        .as_array()
        .ok_or_else(|| Error::Meta(String::from("no alphabet array")))?
        .iter()
        .map(|v| {
            let s = v.as_str().unwrap_or_default();
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Ok(c),
                _ => Err(Error::Meta(format!("alphabet entry {v} is not one character"))),
            }
        })
        .collect::<Result<Vec<char>, Error>>()?;
    let bin = meta["bin"].as_str().unwrap_or("spellcheck.bin");
    let dawg = Dawg::from_bytes(&read(&dir.join(bin))?, alphabet)?;
    let mut speller = Speller::new(dawg);
    if let Some(freq) = meta["freq"].as_str() {
        if let Ok(table) = fs::read_to_string(dir.join(freq)) {
            speller = speller.with_freq(&table);
        }
    }
    Ok(speller)
}

/// Write a pack for `blob` + `alphabet` into `dir` (`spellcheck.bin`, `spellcheck.meta.json`, `SHA256SUMS`), the
/// layout [`load`] reads: what `anbani spell build` produces from a word list.
///
/// # Errors
/// `Io` when the directory or a file cannot be written.
pub fn write(dir: &Path, blob: &[u8], alphabet: &[char], words: usize) -> Result<(), Error> {
    let io = |e: std::io::Error| Error::Io(format!("{}: {e}", dir.display()));
    fs::create_dir_all(dir).map_err(io)?;
    let dawg = Dawg::from_bytes(blob, alphabet.to_vec())?;
    let meta = serde_json::json!({
        "schema": 1, "magic": "ABSC", "format": "csr-dawg", "bin": "spellcheck.bin", "words": words,
        "states": dawg.states(), "edges": dawg.edge_count(), "root": dawg.states() - 1,
        "alphabet": alphabet.iter().map(char::to_string).collect::<Vec<_>>(), "bytes": blob.len(),
        "sha256": sha256_hex(blob),
    });
    let meta = format!("{}\n", serde_json::to_string_pretty(&meta).unwrap_or_default());
    fs::write(dir.join("spellcheck.bin"), blob).map_err(io)?;
    fs::write(dir.join("spellcheck.meta.json"), &meta).map_err(io)?;
    let sums = format!("{}  spellcheck.bin\n{}  spellcheck.meta.json\n", sha256_hex(blob), sha256_hex(meta.as_bytes()));
    fs::write(dir.join("SHA256SUMS"), sums).map_err(io)
}

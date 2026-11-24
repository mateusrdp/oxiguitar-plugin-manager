// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! Putting plugin files in the user's plugin folder, and taking them out.
//! Nothing here touches any other folder.
//!
//! A file is written beside its place under a temporary name and then
//! renamed over it, so a half-written plugin is never left where OxiGuitar
//! looks (it reads only names ending in `.oxp`). A copy OxiGuitar has
//! loaded stays as it is in memory until it quits: Linux and macOS keep
//! the old file's contents for it, and on Windows, where a loaded file
//! can't be replaced or deleted but can be renamed, the old copy is
//! renamed to `NAME.oxp.old` and removed the next time the dialog opens.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use object::read::macho::{FatArch, MachOFatFile32, MachOFatFile64};
use object::{FileKind, Object, ObjectSection};
use oxiguitar_plugin::api::manifest::Manifest;
use oxiguitar_plugin::api::{MANIFEST_SECTION, PE_MANIFEST_SECTION};
use sha2::{Digest, Sha256};

/// A part of a name left by a write that didn't finish.
const PART: &str = ".part";

/// A part of a name a replaced copy is given until it can be removed.
const OLD: &str = ".old";

/// The manifest of the plugin file in `data` (the first build's in a
/// universal macOS file), read as OxiGuitar reads it, without running any
/// of its code.
pub fn manifest_of(data: &[u8]) -> Result<Manifest, String> {
    let not = || "it isn't an OxiGuitar plugin".to_owned();
    let build = match FileKind::parse(data).map_err(|_| not())? {
        FileKind::MachOFat32 => MachOFatFile32::parse(data)
            .ok()
            .and_then(|f| f.arches().first()?.data(data).ok()),
        FileKind::MachOFat64 => MachOFatFile64::parse(data)
            .ok()
            .and_then(|f| f.arches().first()?.data(data).ok()),
        _ => Some(data),
    }
    .ok_or_else(not)?;
    let file = object::File::parse(build).map_err(|_| not())?;
    let section = file
        .section_by_name(MANIFEST_SECTION)
        .or_else(|| file.section_by_name(PE_MANIFEST_SECTION))
        .ok_or_else(not)?;
    let bytes = section.data().map_err(|_| not())?;
    let end = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    Manifest::parse(&bytes[..end]).map_err(|why| format!("its manifest is wrong ({why})"))
}

/// Writes `data` at `to`, replacing what is there.
pub fn replace(to: &Path, data: &[u8]) -> std::io::Result<()> {
    let part = with_suffix(to, PART);
    {
        let mut f = std::fs::File::create(&part)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    // A rename over a file replaces it at once on Linux and macOS; Windows
    // refuses it for a loaded library, but lets that be renamed aside.
    if cfg!(windows) && to.exists() {
        let old = with_suffix(to, OLD);
        let _ = std::fs::remove_file(&old);
        if let Err(e) = std::fs::rename(to, &old) {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        let _ = std::fs::remove_file(&old);
    }
    std::fs::rename(&part, to).inspect_err(|_| {
        let _ = std::fs::remove_file(&part);
    })
}

/// Deletes the plugin file at `path`; on Windows a loaded one is renamed
/// to be removed later.
pub fn delete(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            let old = with_suffix(path, OLD);
            let _ = std::fs::remove_file(&old);
            std::fs::rename(path, &old).map_err(|_| e)
        }
    }
}

/// Removes what earlier writes left in `folder`: unfinished files and
/// replaced copies that were still loaded then.
pub fn clean(folder: &Path) {
    let Ok(dir) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(&format!(".oxp{PART}")) || name.ends_with(&format!(".oxp{OLD}")) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Copies the plugin file `from` into the user's plugin folder `folder`,
/// under its own name, once it is known to be a plugin. Returns where it
/// went and its manifest.
pub fn install_file(from: &Path, folder: &Path) -> Result<(PathBuf, Manifest), String> {
    let name = from
        .file_name()
        .filter(|n| Path::new(n).extension().is_some_and(|e| e == "oxp"))
        .ok_or_else(|| "a plugin file's name ends in .oxp".to_owned())?;
    let data = std::fs::read(from).map_err(|e| format!("it can't be read ({e})"))?;
    let manifest = manifest_of(&data)?;
    let to = folder.join(name);
    if to == from {
        return Err("it is in your plugin folder already".into());
    }
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    replace(&to, &data).map_err(|e| format!("it can't be written to your plugin folder ({e})"))?;
    Ok((to, manifest))
}

/// Checks `data`, downloaded as `name`, against a release's SHA256SUMS.
pub fn check_sum(data: &[u8], name: &str, sums: &str) -> Result<(), String> {
    let want = sums
        .lines()
        .filter_map(|l| l.split_once(char::is_whitespace))
        .find(|(_, file)| file.trim().trim_start_matches('*') == name)
        .map(|(hash, _)| hash.trim().to_ascii_lowercase())
        .ok_or_else(|| format!("the release's SHA256SUMS doesn't list {name}"))?;
    let got: String = Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got == want {
        Ok(())
    } else {
        Err(format!("{name} doesn't match the release's SHA256SUMS"))
    }
}

/// The name a release's plugin file is installed under, as the release's
/// packages install it: `NAME-VERSION-SYSTEM-ARCH.oxp` becomes `NAME.oxp`,
/// and a second build `NAME-VERSION-linux-x86_64-v3.oxp` becomes
/// `NAME-x86-64-v3.oxp` (oxiguitar-packages' `plugin-release.yml`).
pub fn installed_name(file: &str, version: &str, target: &str) -> String {
    let stem = file.strip_suffix(".oxp").unwrap_or(file);
    let (stem, second) = match stem.strip_suffix("-v3") {
        Some(s) => (s, "-x86-64-v3"),
        None => (stem, ""),
    };
    let stem = stem.strip_suffix(&format!("-{target}")).unwrap_or(stem);
    let stem = stem.strip_suffix(&format!("-{version}")).unwrap_or(stem);
    format!("{stem}{second}.oxp")
}

/// The plugin files of the plugin `id` in a release's zip, each with the
/// name it is installed under.
pub fn unpack(
    zip: &[u8],
    id: &str,
    version: &str,
    target: &str,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip))
        .map_err(|e| format!("the release's zip can't be read ({e})"))?;
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("the release's zip can't be read ({e})"))?;
        let Some(name) = entry
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .filter(|n| n.ends_with(".oxp"))
        else {
            continue;
        };
        if entry.size() > crate::releases::DOWNLOAD_MAX {
            return Err(format!("{name} in the release's zip is too large"));
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut data)
            .map_err(|e| format!("{name} in the release's zip can't be read ({e})"))?;
        if manifest_of(&data).is_ok_and(|m| m.id == id) {
            out.push((installed_name(&name, version, target), data));
        }
    }
    if out.is_empty() {
        return Err(format!("the release's zip has no file of {id}"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oxiguitar-pm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_release_s_file_is_installed_under_the_package_s_name() {
        assert_eq!(
            installed_name("gp5-0.3.0-linux-x86_64.oxp", "0.3.0", "linux-x86_64"),
            "gp5.oxp"
        );
        assert_eq!(
            installed_name(
                "nam-1.0.0-beta.2-linux-x86_64-v3.oxp",
                "1.0.0-beta.2",
                "linux-x86_64"
            ),
            "nam-x86-64-v3.oxp"
        );
        assert_eq!(
            installed_name("midi-0.2.0-macos-universal.oxp", "0.2.0", "macos-universal"),
            "midi.oxp"
        );
        // A name made some other way is kept.
        assert_eq!(
            installed_name("odd.oxp", "0.1.0", "linux-x86_64"),
            "odd.oxp"
        );
    }

    #[test]
    fn a_file_replaces_another_whole_and_leaves_nothing_behind() {
        let dir = scratch("replace");
        let to = dir.join("a.oxp");
        replace(&to, b"one").unwrap();
        replace(&to, b"two").unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["a.oxp"]);
        // What a write that stopped half way left is cleaned up.
        std::fs::write(dir.join("b.oxp.part"), b"x").unwrap();
        std::fs::write(dir.join("c.oxp.old"), b"x").unwrap();
        std::fs::write(dir.join("notes.old"), b"x").unwrap();
        clean(&dir);
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a.oxp", "notes.old"]);
        delete(&to).unwrap();
        delete(&to).unwrap();
        assert!(!to.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_a_plugin_file_is_installed() {
        let dir = scratch("install");
        let not = dir.join("song.oxp");
        std::fs::write(&not, b"not a plugin").unwrap();
        let folder = dir.join("plugins");
        assert_eq!(
            install_file(&not, &folder).unwrap_err(),
            "it isn't an OxiGuitar plugin"
        );
        let wrong = dir.join("song.gp5");
        std::fs::write(&wrong, b"x").unwrap();
        assert!(install_file(&wrong, &folder).unwrap_err().contains(".oxp"));
        assert!(!folder.join("song.oxp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_download_is_checked_against_the_release_s_sums() {
        // SHA-256 of "abc".
        let sums = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  x-linux-x86_64.zip\n\
                    0000000000000000000000000000000000000000000000000000000000000000 *y.zip\n";
        assert!(check_sum(b"abc", "x-linux-x86_64.zip", sums).is_ok());
        assert!(
            check_sum(b"abd", "x-linux-x86_64.zip", sums)
                .unwrap_err()
                .contains("doesn't match")
        );
        assert!(check_sum(b"abc", "y.zip", sums).is_err());
        assert!(
            check_sum(b"abc", "z.zip", sums)
                .unwrap_err()
                .contains("doesn't list")
        );
    }

    #[test]
    fn a_zip_without_the_plugin_says_so() {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file(
            "other-0.1.0-linux-x86_64.oxp",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"not a plugin").unwrap();
        let data = zip.finish().unwrap().into_inner();
        assert!(
            unpack(&data, "a.b", "0.1.0", "linux-x86_64")
                .unwrap_err()
                .contains("no file of a.b")
        );
        assert!(unpack(b"not a zip", "a.b", "0.1.0", "linux-x86_64").is_err());
    }
}

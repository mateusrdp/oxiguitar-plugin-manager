// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! A plugin file's parts (mateusrdp/oxiguitar#347): an uncompressed tar
//! archive holding `manifest.json` first and the library second
//! (`plugin.so`, `plugin.dll` or `plugin.dylib`), then whatever else the
//! plugin ships, as OxiGuitar reads it. Nothing is unpacked: both parts
//! are slices of the file's bytes.

use std::io::Read;

use object::FileKind;

/// A plugin file's manifest and library.
#[derive(Debug)]
pub struct Parts<'a> {
    /// `manifest.json`, without trailing white space.
    pub manifest: &'a [u8],
    /// The library.
    pub library: &'a [u8],
}

/// The library names OxiGuitar reads, one for each system.
const LIBRARIES: [&str; 3] = ["plugin.so", "plugin.dll", "plugin.dylib"];

/// The manifest and the library of the plugin file in `data`.
pub fn parts(data: &[u8]) -> Result<Parts<'_>, String> {
    let not = || -> String {
        if FileKind::parse(data).is_ok() {
            "it is a plugin file of the format before OxiGuitar 0.23 (the library alone)".into()
        } else {
            "it isn't an OxiGuitar plugin".into()
        }
    };
    let mut archive = tar::Archive::new(std::io::Cursor::new(data));
    let mut entries = archive.entries().map_err(|_| not())?;
    let mut member = |want: &dyn Fn(&str) -> bool| -> Result<&[u8], String> {
        let mut entry = match entries.next() {
            Some(Ok(entry)) => entry,
            _ => return Err(not()),
        };
        let path = entry.path_bytes().into_owned();
        let name = String::from_utf8_lossy(&path);
        let name = name.strip_prefix("./").unwrap_or(&name);
        if !want(name) || entry.header().entry_type() != tar::EntryType::Regular {
            return Err(not());
        }
        let start = usize::try_from(entry.raw_file_position()).map_err(|_| not())?;
        let end = start
            .checked_add(usize::try_from(entry.size()).map_err(|_| not())?)
            .filter(|&end| end <= data.len())
            .ok_or_else(|| "it is cut short: the file is damaged".to_owned())?;
        // Read past it, so the next header is the next member's.
        std::io::copy(&mut entry.by_ref(), &mut std::io::sink()).map_err(|_| not())?;
        Ok(&data[start..end])
    };
    let manifest = member(&|n| n == "manifest.json")?;
    let library = member(&|n| LIBRARIES.contains(&n))?;
    let end = manifest
        .iter()
        .rposition(|b| !b.is_ascii_whitespace() && *b != 0)
        .map_or(0, |i| i + 1);
    Ok(Parts {
        manifest: &manifest[..end],
        library,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A plugin file of `members` (name, contents), in that order.
    pub(crate) fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, data) in members {
            let mut header = tar::Header::new_ustar();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            builder.append_data(&mut header, name, *data).unwrap();
        }
        builder.into_inner().unwrap()
    }

    #[test]
    fn a_plugin_file_s_manifest_and_library_are_its_first_two_members() {
        let file = archive(&[
            ("manifest.json", b"{\"id\": \"a.b\"}\n"),
            ("plugin.dll", b"code"),
            ("ui/main.slint", b"export component A {}"),
        ]);
        let found = parts(&file).unwrap();
        assert_eq!(found.manifest, b"{\"id\": \"a.b\"}");
        assert_eq!(found.library, b"code");
        for wrong in [
            archive(&[("plugin.so", b"code"), ("manifest.json", b"{}")]),
            archive(&[("manifest.json", b"{}")]),
            archive(&[("manifest.json", b"{}"), ("ui/main.slint", b"")]),
            b"not a plugin".to_vec(),
        ] {
            assert_eq!(parts(&wrong).unwrap_err(), "it isn't an OxiGuitar plugin");
        }
        let mut cut = archive(&[("manifest.json", b"{}"), ("plugin.so", &[1u8; 3000])]);
        cut.truncate(2000);
        assert!(parts(&cut).unwrap_err().contains("cut short"));
    }

    #[test]
    fn a_bare_library_is_the_old_format() {
        let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        assert!(parts(&me).unwrap_err().contains("before OxiGuitar 0.23"));
    }
}

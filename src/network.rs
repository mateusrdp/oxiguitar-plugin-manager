// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! Whether a plugin file can reach the network, from the functions it
//! imports (Mateus, 24 Nov 2026: no manifest key, which authors wouldn't
//! fill in and could get wrong; only a quick look when the window opens).
//!
//! A plugin is a shared library, and the system's network calls live in
//! the system's libraries: a plugin that opens a connection imports
//! `socket` and `connect` (or `getaddrinfo`) from the C library on Linux
//! and macOS, the Winsock functions from `ws2_32.dll` on Windows, or one of
//! the systems' HTTP libraries (WinHTTP, WinINet, the macOS Network and
//! CFNetwork frameworks). Rust links only the code a library uses, so a
//! plugin that never connects imports none of them. It is a strong hint,
//! not a proof: a plugin could find those functions at run time, or make
//! the system calls itself, and the manual says so.

use std::path::Path;

use object::read::macho::{FatArch, MachOFatFile32, MachOFatFile64};
use object::{FileKind, NameOrOrdinal, Object};

/// The functions that open a connection or look up a name, as the C
/// library and Winsock name them.
const CALLS: [&str; 7] = [
    "socket",
    "connect",
    "getaddrinfo",
    "WSASocketW",
    "WSASocketA",
    "WSAConnect",
    "GetAddrInfoW",
];

/// System libraries whose every import is about the network.
const LIBRARIES: [&str; 4] = [
    "winhttp.dll",
    "wininet.dll",
    "Network.framework",
    "CFNetwork",
];

/// Whether an imported function `name` from `library` (empty when the
/// format doesn't say, as in ELF) is a network call.
pub fn is_network_import(library: &str, name: &str) -> bool {
    // Mach-O symbols start with an underscore.
    let name = name.strip_prefix('_').unwrap_or(name);
    CALLS.contains(&name)
        || LIBRARIES.iter().any(|l| {
            library
                .to_ascii_lowercase()
                .contains(&l.to_ascii_lowercase())
        })
}

/// Whether the plugin file in `data` imports a network call (its library,
/// #347); a universal macOS file when any of its builds does.
pub fn uses_network(data: &[u8]) -> Result<bool, String> {
    library_uses_network(crate::oxp::parts(data)?.library)
}

/// Whether the library in `data` imports a network call.
fn library_uses_network(data: &[u8]) -> Result<bool, String> {
    let kind = FileKind::parse(data).map_err(|_| "not a library file".to_owned())?;
    match kind {
        FileKind::MachOFat32 => {
            let fat = MachOFatFile32::parse(data).map_err(|e| e.to_string())?;
            any_build(data, fat.arches())
        }
        FileKind::MachOFat64 => {
            let fat = MachOFatFile64::parse(data).map_err(|e| e.to_string())?;
            any_build(data, fat.arches())
        }
        _ => one(data),
    }
}

fn any_build<A: FatArch>(data: &[u8], arches: &[A]) -> Result<bool, String> {
    for arch in arches {
        let build = arch.data(data).map_err(|e| e.to_string())?;
        if one(build)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn one(data: &[u8]) -> Result<bool, String> {
    let file = object::File::parse(data).map_err(|e| e.to_string())?;
    for import in file.imports().map_err(|e| e.to_string())? {
        let i = import.map_err(|e| e.to_string())?;
        // A function imported by number (Windows) has no name to know it
        // by; its library still tells.
        let name = match i.name() {
            NameOrOrdinal::Name(n) => String::from_utf8_lossy(n).into_owned(),
            NameOrOrdinal::Ordinal(_) => String::new(),
        };
        if is_network_import(&String::from_utf8_lossy(i.library()), &name) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the plugin file at `path` can reach the network.
pub fn file_uses_network(path: &Path) -> Result<bool, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    uses_network(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_calls_are_told_by_name_and_library() {
        assert!(is_network_import("", "socket"));
        assert!(is_network_import("", "getaddrinfo"));
        assert!(is_network_import("/usr/lib/libSystem.B.dylib", "_connect"));
        assert!(is_network_import("WS2_32.dll", "WSASocketW"));
        assert!(is_network_import("WINHTTP.dll", "WinHttpOpen"));
        assert!(is_network_import(
            "/System/Library/Frameworks/Network.framework/Versions/A/Network",
            "_nw_connection_create"
        ));
        for (library, name) in [
            ("", "malloc"),
            ("", "write"),
            ("KERNEL32.dll", "CreateFileW"),
            ("/usr/lib/libSystem.B.dylib", "_pthread_create"),
            ("", "socketpair"),
        ] {
            assert!(!is_network_import(library, name), "{library} {name}");
        }
    }

    #[test]
    fn what_isnt_a_library_says_so() {
        assert!(uses_network(b"not a plugin").is_err());
    }
}

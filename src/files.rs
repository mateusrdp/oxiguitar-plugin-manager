// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! What OxiGuitar found (`Host::plugin_files`, interface 1.15), as the
//! dialog lists it: every plugin file, every copy of a plugin included,
//! with what the user may do with it.
//!
//! A copy in a system folder (where packages install) is shown but never
//! deleted (#344): the package manager owns it. A copy beside the program
//! (a portable copy's folder) or in a folder settings.json lists may be,
//! when the folder can be written; a copy inside the macOS app may not,
//! since changing the app's files breaks its signature.

use std::path::{Path, PathBuf};

use oxiguitar_plugin::{PluginFile, PluginFileState, PluginFolderKind};

/// A plugin file, as the window shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The file.
    pub path: PathBuf,
    /// What kind of folder holds it.
    pub folder: PluginFolderKind,
    /// The plugin's id; `None` when its manifest can't be read.
    pub id: Option<String>,
    /// Its name, or the file's name when the manifest can't be read.
    pub name: String,
    /// Its version, when the manifest gives one.
    pub version: Option<semver::Version>,
    /// Its homepage.
    pub homepage: String,
    /// Whether OxiGuitar uses it.
    pub state: PluginFileState,
    /// Whether the user may delete it here.
    pub deletable: bool,
}

impl Entry {
    /// The list's entry for a file OxiGuitar found.
    pub fn of(file: PluginFile) -> Entry {
        let path = PathBuf::from(&file.path);
        let name = if file.name.is_empty() {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| file.path.clone())
        } else {
            file.name
        };
        let deletable = may_delete(file.folder, cfg!(target_os = "macos"))
            && (file.folder == PluginFolderKind::User || folder_writable(&path));
        Entry {
            folder: file.folder,
            id: file.id,
            name,
            version: semver::Version::parse(&file.version).ok(),
            homepage: file.homepage,
            state: file.state,
            deletable,
            path,
        }
    }

    /// Whether OxiGuitar uses this copy.
    pub fn used(&self) -> bool {
        self.state == PluginFileState::Used
    }

    /// The folder's kind, in a word.
    pub fn folder_text(&self) -> &'static str {
        match self.folder {
            PluginFolderKind::User => "your folder",
            PluginFolderKind::Program => "beside OxiGuitar",
            PluginFolderKind::System => "system",
            PluginFolderKind::Listed => "listed folder",
        }
    }

    /// Whether OxiGuitar uses it, in a few words.
    pub fn state_text(&self) -> String {
        match &self.state {
            PluginFileState::Used => "in use".into(),
            PluginFileState::Disabled => "switched off".into(),
            PluginFileState::Replaced(by) => {
                let by = Path::new(by);
                match by.file_name() {
                    Some(n) => format!("not used: {} is", n.to_string_lossy()),
                    None => "not used: another copy is".into(),
                }
            }
            PluginFileState::Unusable(why)
            | PluginFileState::Unmet(why)
            | PluginFileState::Withdrawn(why) => format!("can't be used: {why}"),
        }
    }
}

/// Whether a copy in a folder of `kind` may be deleted from the window
/// (`macos` for the macOS app, whose own folder is inside its bundle).
pub fn may_delete(kind: PluginFolderKind, macos: bool) -> bool {
    match kind {
        PluginFolderKind::User | PluginFolderKind::Listed => true,
        PluginFolderKind::Program => !macos,
        PluginFolderKind::System => false,
    }
}

/// Whether the folder holding `path` can be written: a portable copy's can,
/// an AppImage's (mounted read only) or a package's can't. Tried by
/// making and removing an empty file, since permissions alone don't show a
/// read-only mount.
pub fn folder_writable(path: &Path) -> bool {
    let Some(folder) = path.parent() else {
        return false;
    };
    let probe = folder.join(format!(".oxiguitar-plugin-manager-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// The newest version among the copies of `id`.
pub fn newest_installed<'a>(
    entries: impl IntoIterator<Item = &'a Entry>,
    id: &str,
) -> Option<&'a semver::Version> {
    entries
        .into_iter()
        .filter(|e| e.id.as_deref() == Some(id))
        .filter_map(|e| e.version.as_ref())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, folder: PluginFolderKind, version: &str) -> PluginFile {
        PluginFile {
            path: path.into(),
            folder,
            id: Some("someone.thing".into()),
            name: "Thing".into(),
            version: version.into(),
            homepage: "https://github.com/someone/thing".into(),
            state: PluginFileState::Used,
            loaded: false,
        }
    }

    #[test]
    fn system_copies_are_never_deleted_and_the_macos_app_s_neither() {
        assert!(may_delete(PluginFolderKind::User, false));
        assert!(may_delete(PluginFolderKind::User, true));
        assert!(may_delete(PluginFolderKind::Listed, false));
        assert!(may_delete(PluginFolderKind::Program, false));
        assert!(!may_delete(PluginFolderKind::Program, true));
        assert!(!may_delete(PluginFolderKind::System, false));
        assert!(!may_delete(PluginFolderKind::System, true));
    }

    #[test]
    fn every_copy_is_an_entry_and_the_newest_version_is_known() {
        let entries = [
            Entry::of(file(
                "/usr/lib/oxiguitar/plugins/thing.oxp",
                PluginFolderKind::System,
                "0.2.0",
            )),
            Entry::of(file(
                "/home/u/plugins/thing.oxp",
                PluginFolderKind::User,
                "0.3.0-beta.1",
            )),
            Entry::of(PluginFile {
                id: None,
                name: String::new(),
                version: String::new(),
                state: PluginFileState::Unusable("damaged".into()),
                ..file("/home/u/plugins/broken.oxp", PluginFolderKind::User, "")
            }),
        ];
        assert!(!entries[0].deletable && entries[1].deletable);
        assert_eq!(entries[2].name, "broken.oxp");
        assert_eq!(entries[2].version, None);
        assert_eq!(entries[2].state_text(), "can't be used: damaged");
        assert_eq!(
            newest_installed(&entries, "someone.thing").map(ToString::to_string),
            Some("0.3.0-beta.1".into())
        );
        assert_eq!(newest_installed(&entries, "nobody"), None);
        let replaced = Entry::of(PluginFile {
            state: PluginFileState::Replaced("/home/u/plugins/thing.oxp".into()),
            ..file(
                "/usr/lib/oxiguitar/plugins/thing.oxp",
                PluginFolderKind::System,
                "0.2.0",
            )
        });
        assert_eq!(replaced.state_text(), "not used: thing.oxp is");
    }

    #[test]
    fn a_folder_that_can_be_written_is_told_apart() {
        let dir =
            std::env::temp_dir().join(format!("oxiguitar-pm-writable-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(folder_writable(&dir.join("a.oxp")));
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_none(),
            "the probe is gone"
        );
        assert!(!folder_writable(&dir.join("missing").join("a.oxp")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

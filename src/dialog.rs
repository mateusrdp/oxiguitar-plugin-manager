// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! Plugins → Plugin Manager… and the dialog it opens (#344).
//!
//! Opening it asks OxiGuitar for every plugin file it found when it
//! started (`Host::plugin_files`), and lists them, a row a copy, grouped by
//! plugin: name, version, folder, whether OxiGuitar uses that copy, and a
//! `!` when the file can reach the network. Then, off OxiGuitar's threads,
//! it looks at each file's imports (`network`) and asks GitHub for each
//! plugin's newest release (`releases`), redrawing as answers come.
//!
//! What the user does happens at once, in the user's plugin folder only:
//! **Update** downloads the newest release's file for this system, checks
//! it against the release's SHA256SUMS, and puts it there (replacing the
//! user's older copies of that plugin); **Install a Plugin File…** copies
//! a `.oxp` the user chooses (in the system's file dialog, the plugin's
//! own) there; **Delete** removes a copy that isn't in a system folder.
//! OxiGuitar reads its plugin folders once, when it starts, so each change
//! is used from the next start; the dialog says so. OK and Cancel both
//! just close it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use oxiguitar_plugin::{Error, Host, Row, Tab, TabContent, TabEvent, Tool, Widget};

use crate::files::{self, Entry};
use crate::install;
use crate::network;
use crate::releases::{self, Checker, Found, Repo};

/// The dialog's id in the manifest.
pub const DIALOG: &str = "manager";

/// A copy in the dialog.
#[derive(Clone, Debug)]
struct Copy {
    entry: Entry,
    /// Whether it can reach the network; `None` until looked at.
    network: Option<Result<bool, String>>,
    /// What happened to it since the dialog opened ("deleted").
    note: Option<String>,
}

/// A file put in the user's folder since the dialog opened.
#[derive(Clone, Debug, PartialEq)]
struct Added {
    name: String,
    version: String,
    path: PathBuf,
}

/// What the dialog shows.
#[derive(Debug, Default)]
struct State {
    /// Each time the dialog opens; work started for an earlier opening
    /// keeps its answers to itself.
    generation: u64,
    user_folder: PathBuf,
    copies: Vec<Copy>,
    added: Vec<Added>,
    /// Each repository's newest release; `None` while it is asked.
    latest: HashMap<Repo, Option<Found>>,
    /// The plugin being updated.
    updating: Option<String>,
    /// The last thing done, or what went wrong.
    message: Option<String>,
}

impl State {
    /// The plugins in the dialog's order (first seen first), each with its
    /// copies' indexes; a file whose manifest can't be read is its own.
    fn groups(&self) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = Vec::new();
        let mut by_id: HashMap<&str, usize> = HashMap::new();
        for (i, c) in self.copies.iter().enumerate() {
            match c.entry.id.as_deref() {
                Some(id) => match by_id.get(id) {
                    Some(&g) => out[g].push(i),
                    None => {
                        by_id.insert(id, out.len());
                        out.push(vec![i]);
                    }
                },
                None => out.push(vec![i]),
            }
        }
        out
    }

    /// The repository of plugin `id`, from its copies' homepages.
    fn repo(&self, id: &str) -> Option<Repo> {
        self.copies
            .iter()
            .filter(|c| c.entry.id.as_deref() == Some(id))
            .find_map(|c| releases::repo_of(&c.entry.homepage))
    }

    /// The newest release of `id` when it is newer than every copy.
    fn update_for(&self, id: &str) -> Option<releases::Release> {
        let repo = self.repo(id)?;
        let Some(Some(Found::Release(r))) = self.latest.get(&repo) else {
            return None;
        };
        let installed = files::newest_installed(self.copies.iter().map(|c| &c.entry), id);
        installed.is_none_or(|v| r.version > *v).then(|| r.clone())
    }

    /// What the dialog says about `id`'s newest release.
    fn latest_text(&self, id: &str) -> String {
        let Some(repo) = self.repo(id) else {
            return "not on GitHub: can't check".into();
        };
        match self.latest.get(&repo) {
            None | Some(None) => "checking…".into(),
            Some(Some(Found::Nothing)) => "no release for this system".into(),
            Some(Some(Found::Failed(why))) => format!("can't check: {why}"),
            Some(Some(Found::Release(r))) => match self.update_for(id) {
                Some(_) => format!("newest: {}", r.version),
                None => "up to date".into(),
            },
        }
    }
}

/// What the tool, the dialog and the work off their threads share.
#[derive(Debug)]
pub struct Shared {
    host: Host,
    checker: Checker,
    state: Mutex<State>,
}

impl Shared {
    /// Nothing listed yet.
    pub fn new(host: Host) -> Shared {
        Shared {
            host,
            checker: Checker::new(),
            state: Mutex::new(State::default()),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Changes the state if the dialog is still the one opened as
    /// `generation`, then shows it again.
    fn update(&self, generation: u64, change: impl FnOnce(&mut State)) {
        {
            let mut state = self.state();
            if state.generation != generation {
                return;
            }
            change(&mut state);
        }
        self.host.redraw(DIALOG);
    }
}

/// Opens the dialog, listing what OxiGuitar found, and starts looking at
/// the files and asking GitHub.
pub fn open(shared: &Arc<Shared>) -> Result<String, Error> {
    let found = match shared.host.plugin_files() {
        Ok(f) => f,
        Err(e) => return Ok(format!("The plugin files can't be listed: {}.", e.message)),
    };
    let user_folder = PathBuf::from(&found.user_folder);
    install::clean(&user_folder);
    let copies: Vec<Copy> = found
        .files
        .into_iter()
        .map(|f| Copy {
            entry: Entry::of(f),
            network: None,
            note: None,
        })
        .collect();
    let repos: Vec<Repo> = {
        let mut repos: Vec<Repo> = copies
            .iter()
            .filter(|c| c.entry.id.is_some())
            .filter_map(|c| releases::repo_of(&c.entry.homepage))
            .collect();
        repos.sort_by_key(ToString::to_string);
        repos.dedup();
        repos
    };
    let paths: Vec<PathBuf> = copies.iter().map(|c| c.entry.path.clone()).collect();
    let generation = {
        let mut state = shared.state();
        let generation = state.generation + 1;
        *state = State {
            generation,
            user_folder,
            copies,
            latest: repos.iter().map(|r| (r.clone(), None)).collect(),
            ..State::default()
        };
        generation
    };
    shared.host.open_dialog(DIALOG)?;
    let work = shared.clone();
    std::thread::Builder::new()
        .name("plugin-manager".into())
        .spawn(move || look(&work, generation, &paths, &repos))
        .map_err(|e| Error::new(oxiguitar_plugin::ErrorKind::Failed, e.to_string()))?;
    Ok(String::new())
}

/// Off OxiGuitar's threads: each file's imports, then every repository's
/// releases, all at once.
fn look(shared: &Arc<Shared>, generation: u64, paths: &[PathBuf], repos: &[Repo]) {
    let marks: Vec<_> = paths
        .iter()
        .map(|p| network::file_uses_network(p))
        .collect();
    shared.update(generation, |s| {
        for (c, m) in s.copies.iter_mut().zip(marks) {
            c.network = Some(m);
        }
    });
    std::thread::scope(|scope| {
        for repo in repos {
            scope.spawn(move || {
                let found = shared.checker.check(repo);
                shared.update(generation, |s| {
                    s.latest.insert(repo.clone(), Some(found));
                });
            });
        }
    });
}

/// Puts the newest release of `id` in the user's folder: downloaded,
/// checked, unpacked, written; the user's older copies go.
fn update(shared: &Shared, generation: u64, id: &str, release: &releases::Release) {
    let user_folder = shared.state().user_folder.clone();
    let result = (|| -> Result<Vec<PathBuf>, String> {
        let zip = shared.checker.download(&release.zip)?;
        if let Some(sums) = &release.sums {
            let sums = shared.checker.download(sums)?;
            install::check_sum(&zip, &release.zip_name, &String::from_utf8_lossy(&sums))?;
        }
        let version = release.version.to_string();
        let files = install::unpack(&zip, id, &version, releases::target())?;
        std::fs::create_dir_all(&user_folder).map_err(|e| e.to_string())?;
        let mut written = Vec::new();
        for (name, data) in files {
            let to = user_folder.join(&name);
            install::replace(&to, &data)
                .map_err(|e| format!("{name} can't be written to your plugin folder ({e})"))?;
            written.push(to);
        }
        Ok(written)
    })();
    shared.update(generation, |s| {
        s.updating = None;
        let name = s
            .copies
            .iter()
            .find(|c| c.entry.id.as_deref() == Some(id))
            .map_or_else(|| id.to_owned(), |c| c.entry.name.clone());
        match result {
            Ok(written) => {
                let version = release.version.to_string();
                for c in &mut s.copies {
                    if c.entry.id.as_deref() != Some(id) || c.note.is_some() {
                        continue;
                    }
                    if written.contains(&c.entry.path) {
                        c.note = Some(format!("replaced by {version}"));
                    } else if c.entry.path.parent() == Some(user_folder.as_path()) {
                        // The user's older copy goes: the new one wins anyway.
                        if install::delete(&c.entry.path).is_ok() {
                            c.note = Some("deleted".into());
                        }
                    }
                }
                for path in written {
                    s.added.retain(|a| a.path != path);
                    s.added.push(Added {
                        name: name.clone(),
                        version: version.clone(),
                        path,
                    });
                }
                s.message = Some(format!(
                    "{name} {version} is in your plugin folder. Start OxiGuitar again to use it."
                ));
            }
            Err(why) => s.message = Some(format!("{name} can't be updated: {why}.")),
        }
    });
}

/// Plugins → Plugin Manager….
#[derive(Debug)]
pub struct ManageTool {
    /// What it shares with the dialog.
    pub shared: Arc<Shared>,
}

impl Tool for ManageTool {
    fn run(&self) -> Result<String, Error> {
        open(&self.shared)
    }
}

/// The Plugin Manager dialog.
#[derive(Debug)]
pub struct Dialog {
    /// What it shares with the tool.
    pub shared: Arc<Shared>,
}

fn label(text: impl Into<String>, muted: bool) -> Widget {
    Widget::Label {
        text: text.into(),
        muted,
    }
}

fn button(id: impl Into<String>, text: impl Into<String>, enabled: bool) -> Widget {
    Widget::Button {
        id: id.into(),
        text: text.into(),
        enabled,
    }
}

/// A copy's name, with `!` when it can reach the network.
fn name_text(c: &Copy) -> String {
    match c.network {
        Some(Ok(true)) => format!("{} !", c.entry.name),
        _ => c.entry.name.clone(),
    }
}

/// A copy's version, folder and state.
fn about_text(c: &Copy) -> String {
    let version = c
        .entry
        .version
        .as_ref()
        .map_or_else(|| "no version".to_owned(), ToString::to_string);
    let state = c.note.clone().unwrap_or_else(|| c.entry.state_text());
    format!("{version} · {} · {state}", c.entry.folder_text())
}

impl Tab for Dialog {
    fn content(&self) -> TabContent {
        let s = self.shared.state();
        if s.generation == 0 {
            return TabContent {
                rows: vec![Row {
                    widgets: vec![label("Open it from Plugins → Plugin Manager….", true)],
                }],
            };
        }
        let mut rows = vec![Row {
            widgets: vec![label(
                "Every plugin file OxiGuitar found when it started, each copy on its own line. A ! after a name: the plugin can reach the network (it uses the system's network functions). Changes here are used when you start OxiGuitar again.",
                true,
            )],
        }];
        for group in s.groups() {
            let first = &s.copies[group[0]];
            let id = first.entry.id.clone();
            let update = id.as_deref().and_then(|id| s.update_for(id));
            for (n, &i) in group.iter().enumerate() {
                let c = &s.copies[i];
                let system = !c.entry.deletable
                    && c.entry.folder == oxiguitar_plugin::PluginFolderKind::System;
                let mut widgets = vec![
                    label(name_text(c), system || c.note.is_some()),
                    label(about_text(c), true),
                ];
                if n == 0
                    && let Some(id) = &id
                {
                    widgets.push(label(s.latest_text(id), true));
                    if let Some(r) = &update {
                        widgets.push(button(
                            format!("update:{id}"),
                            format!("Update to {}", r.version),
                            s.updating.is_none(),
                        ));
                    }
                }
                widgets.push(button(
                    format!("delete:{i}"),
                    "Delete",
                    c.entry.deletable && c.note.as_deref() != Some("deleted"),
                ));
                rows.push(Row { widgets });
            }
        }
        for a in &s.added {
            rows.push(Row {
                widgets: vec![
                    label(&a.name, false),
                    label(
                        format!("{} · your folder · used after a restart", a.version),
                        true,
                    ),
                ],
            });
        }
        rows.push(Row {
            widgets: vec![button(
                "install",
                "Install a Plugin File…",
                s.updating.is_none(),
            )],
        });
        if let Some(name) = &s.updating {
            rows.push(Row {
                widgets: vec![label(format!("Updating {name}…"), false)],
            });
        }
        if let Some(m) = &s.message {
            rows.push(Row {
                widgets: vec![label(m, false)],
            });
        }
        TabContent { rows }
    }

    fn on(&self, widget: &str, event: TabEvent) -> Result<(), Error> {
        if event != TabEvent::Clicked {
            return Ok(());
        }
        if widget == "install" {
            self.install();
        } else if let Some(id) = widget.strip_prefix("update:") {
            self.start_update(id)?;
        } else if let Some(i) = widget.strip_prefix("delete:")
            && let Ok(i) = i.parse::<usize>()
        {
            self.delete(i);
        }
        Ok(())
    }

    fn closed(&self, _ok: bool) -> Result<(), Error> {
        // What was done is done; work still running keeps its answers.
        let mut s = self.shared.state();
        s.generation += 1;
        s.copies.clear();
        s.added.clear();
        s.updating = None;
        s.message = None;
        Ok(())
    }
}

impl Dialog {
    fn start_update(&self, id: &str) -> Result<(), Error> {
        let (generation, release, name) = {
            let mut s = self.shared.state();
            let Some(release) = s.update_for(id) else {
                return Ok(());
            };
            if s.updating.is_some() {
                return Ok(());
            }
            let name = s
                .copies
                .iter()
                .find(|c| c.entry.id.as_deref() == Some(id))
                .map_or_else(|| id.to_owned(), |c| c.entry.name.clone());
            s.updating = Some(name.clone());
            s.message = None;
            (s.generation, release, name)
        };
        let shared = self.shared.clone();
        let id = id.to_owned();
        std::thread::Builder::new()
            .name("plugin-manager-update".into())
            .spawn(move || update(&shared, generation, &id, &release))
            .map(|_| ())
            .map_err(|e| {
                self.shared.state().updating = None;
                Error::new(
                    oxiguitar_plugin::ErrorKind::Failed,
                    format!("{name} can't be updated: {e}"),
                )
            })
    }

    fn delete(&self, i: usize) {
        let mut s = self.shared.state();
        let Some(c) = s.copies.get(i) else {
            return;
        };
        if !c.entry.deletable || c.note.as_deref() == Some("deleted") {
            return;
        }
        let path = c.entry.path.clone();
        let name = c.entry.name.clone();
        match install::delete(&path) {
            Ok(()) => {
                s.copies[i].note = Some("deleted".into());
                s.message = Some(format!(
                    "{name} ({}) is deleted. OxiGuitar stops using it when you start it again.",
                    display_path(&path)
                ));
            }
            Err(e) => {
                s.message = Some(format!("{name} can't be deleted: {e}."));
            }
        }
    }

    /// The system's file dialog for a `.oxp` file, which is copied into the
    /// user's plugin folder.
    fn install(&self) {
        let Some(file) = rfd::FileDialog::new()
            .set_title("Install a Plugin File")
            .add_filter("OxiGuitar plugins", &["oxp"])
            .pick_file()
        else {
            return;
        };
        let mut s = self.shared.state();
        let folder = s.user_folder.clone();
        match install::install_file(&file, &folder) {
            Ok((to, manifest)) => {
                let version = manifest.version.to_string();
                for c in &mut s.copies {
                    if c.entry.path == to {
                        c.note = Some(format!("replaced by {version}"));
                    }
                }
                s.added.retain(|a| a.path != to);
                s.added.push(Added {
                    name: manifest.name.clone(),
                    version: version.clone(),
                    path: to,
                });
                s.message = Some(format!(
                    "{} {version} is in your plugin folder. Start OxiGuitar again to use it.",
                    manifest.name
                ));
            }
            Err(why) => {
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                s.message = Some(format!("{name} can't be installed: {why}."));
            }
        }
    }
}

/// A path for a message.
fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxiguitar_plugin::api::message::{HostAnswer, HostRequest};
    use oxiguitar_plugin::{
        HostServices, PluginFile, PluginFileState, PluginFiles, PluginFolderKind,
    };

    /// A host with a list of plugin files, counting redraws and dialogs.
    #[derive(Default)]
    struct Listed(Mutex<Vec<String>>, PluginFiles);

    impl HostServices for Listed {
        fn request(&self, request: HostRequest<'_>) -> Result<HostAnswer<'static>, Error> {
            match request {
                HostRequest::PluginFiles => Ok(HostAnswer::PluginFiles(self.1.clone())),
                HostRequest::OpenDialog { tab } => {
                    self.0.lock().unwrap().push(format!("open {tab}"));
                    Ok(HostAnswer::Done)
                }
                _ => Ok(HostAnswer::Done),
            }
        }
    }

    fn file(path: &Path, folder: PluginFolderKind, id: &str, version: &str) -> PluginFile {
        PluginFile {
            path: path.to_str().unwrap().into(),
            folder,
            id: Some(id.into()),
            name: id.into(),
            version: version.into(),
            // Not GitHub: the test asks nobody.
            homepage: "https://example.org".into(),
            state: PluginFileState::Used,
            loaded: false,
        }
    }

    fn texts(content: &TabContent) -> Vec<String> {
        content
            .rows
            .iter()
            .map(|r| {
                r.widgets
                    .iter()
                    .map(|w| match w {
                        Widget::Label { text, .. } => text.clone(),
                        Widget::Button { id, enabled, .. } => {
                            format!("[{id}{}]", if *enabled { "" } else { " off" })
                        }
                        _ => "?".into(),
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            })
            .collect()
    }

    #[test]
    fn every_copy_is_listed_and_only_the_users_are_deleted() {
        let dir = std::env::temp_dir().join(format!("oxiguitar-pm-dialog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let user = dir.join("plugins");
        std::fs::create_dir_all(&user).unwrap();
        let mine = user.join("a.oxp");
        std::fs::write(&mine, b"not a library").unwrap();
        let system = Path::new("/usr/lib/oxiguitar/plugins/a.oxp");
        let host = Arc::new(Listed(
            Mutex::default(),
            PluginFiles {
                user_folder: user.to_str().unwrap().into(),
                files: vec![
                    PluginFile {
                        state: PluginFileState::Replaced(mine.to_str().unwrap().into()),
                        ..file(system, PluginFolderKind::System, "a.b", "0.1.0")
                    },
                    file(&mine, PluginFolderKind::User, "a.b", "0.2.0"),
                ],
            },
        ));
        let shared = Arc::new(Shared::new(Host::new(host.clone())));
        let dialog = Dialog {
            shared: shared.clone(),
        };
        assert_eq!(texts(&dialog.content()).len(), 1, "nothing until it opens");
        assert_eq!(open(&shared).unwrap(), "");
        assert_eq!(*host.0.lock().unwrap(), ["open manager"]);
        let shown = texts(&dialog.content());
        assert_eq!(
            shown[1],
            "a.b | 0.1.0 · system · not used: a.oxp is | not on GitHub: can't check | [delete:0 off]"
        );
        assert_eq!(shown[2], "a.b | 0.2.0 · your folder · in use | [delete:1]");
        assert_eq!(shown[3], "[install]");
        // A system copy stays whatever is asked.
        dialog.on("delete:0", TabEvent::Clicked).unwrap();
        dialog.on("delete:1", TabEvent::Clicked).unwrap();
        assert!(!mine.exists());
        let shown = texts(&dialog.content());
        assert_eq!(
            shown[2],
            "a.b | 0.2.0 · your folder · deleted | [delete:1 off]"
        );
        assert!(shown[4].contains("is deleted"), "{shown:?}");
        dialog.closed(true).unwrap();
        assert_eq!(
            texts(&dialog.content()).len(),
            2,
            "closed: the hint and the button"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_update_is_offered_only_when_a_release_is_newer() {
        let repo = Repo {
            owner: "a".into(),
            name: "b".into(),
        };
        let entry = |version: &str| Copy {
            entry: Entry::of(PluginFile {
                homepage: "https://github.com/a/b".into(),
                ..file(
                    Path::new("/x/a.oxp"),
                    PluginFolderKind::System,
                    "a.b",
                    version,
                )
            }),
            network: Some(Ok(true)),
            note: None,
        };
        let release = |v: &str| releases::Release {
            version: semver::Version::parse(v).unwrap(),
            zip_name: format!("a-{v}-linux-x86_64.zip"),
            zip: "https://github.com/a/b/z.zip".into(),
            sums: None,
        };
        let mut s = State {
            generation: 1,
            copies: vec![entry("0.2.0")],
            latest: [(repo.clone(), None)].into(),
            ..State::default()
        };
        assert_eq!(s.latest_text("a.b"), "checking…");
        assert_eq!(name_text(&s.copies[0]), "a.b !");
        s.latest
            .insert(repo.clone(), Some(Found::Release(release("0.2.0"))));
        assert_eq!(s.latest_text("a.b"), "up to date");
        assert!(s.update_for("a.b").is_none());
        s.latest
            .insert(repo.clone(), Some(Found::Release(release("0.3.0-beta.1"))));
        assert_eq!(s.latest_text("a.b"), "newest: 0.3.0-beta.1");
        assert_eq!(
            s.update_for("a.b").unwrap().version.to_string(),
            "0.3.0-beta.1"
        );
        s.latest.insert(
            repo.clone(),
            Some(Found::Failed("GitHub answered 500".into())),
        );
        assert_eq!(s.latest_text("a.b"), "can't check: GitHub answered 500");
        s.latest.insert(repo, Some(Found::Nothing));
        assert_eq!(s.latest_text("a.b"), "no release for this system");
    }
}

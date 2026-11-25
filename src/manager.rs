// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! What the window shows and what its buttons do, apart from the UI
//! (`ui`), so it is tested without one.
//!
//! Opening the window asks OxiGuitar for every plugin file it found when it
//! started (`Host::plugin_files`). Then, off OxiGuitar's threads, it
//! looks at each file's imports (`network`) and asks GitHub for each
//! plugin's newest release (`releases`).
//!
//! What the user does happens at once, and only in the user's plugin folder
//! or to a copy that isn't in a system folder: **Add…** copies a `.oxp`
//! there, **Update** puts the newest release's file where the copy
//! OxiGuitar uses is, when that folder can be written, or else in the
//! user's folder (replacing the older copies in that folder), **X** deletes
//! a copy once confirmed. OxiGuitar uses the copy in the folder it searches
//! last, whatever the versions (mateusrdp/oxiguitar#347): beside the
//! program, then the user's, then the system's. OxiGuitar
//! reads its plugin folders when it starts, so each change is used from
//! the next start.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use oxiguitar_plugin::{Error, Host, PluginFileState, PluginFolderKind};

use crate::files::{self, Entry};
use crate::install;
use crate::network;
use crate::releases::{self, Checker, Found, Release, Repo};

/// A copy in the list.
#[derive(Clone, Debug)]
pub struct Copy {
    /// What OxiGuitar found, or what was added since.
    pub entry: Entry,
    /// Whether it can reach the network, once looked at.
    pub network: bool,
}

/// What the window shows.
#[derive(Debug, Default)]
pub struct State {
    /// Each time the window opens; work started for an earlier opening
    /// keeps its answers to itself.
    pub generation: u64,
    /// The user's plugin folder.
    pub user_folder: PathBuf,
    /// The copies, those of a plugin together, in the order OxiGuitar
    /// found the plugins.
    pub copies: Vec<Copy>,
    /// Each repository's newest release, once GitHub answered.
    pub latest: HashMap<Repo, Found>,
    /// An update is being downloaded.
    pub busy: bool,
    /// What the last thing done came to; empty: nothing to say.
    pub status: String,
}

/// A line of the list, as the window shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineData {
    /// The plugin's name, after `! ` when it can reach the network.
    pub name: String,
    /// Its version; empty when its manifest can't be read.
    pub version: String,
    /// The file.
    pub path: String,
    /// It may be deleted.
    pub removable: bool,
    /// The newest release when it is newer than every copy, on the
    /// plugin's first line; empty otherwise.
    pub update: String,
}

impl State {
    /// The newest release of `id` when it is newer than every copy.
    pub fn update_for(&self, id: &str) -> Option<Release> {
        let repo = self.repo(id)?;
        let Some(Found::Release(r)) = self.latest.get(&repo) else {
            return None;
        };
        let installed = files::newest_installed(self.copies.iter().map(|c| &c.entry), id);
        installed.is_none_or(|v| r.version > *v).then(|| r.clone())
    }

    fn repo(&self, id: &str) -> Option<Repo> {
        self.copies
            .iter()
            .filter(|c| c.entry.id.as_deref() == Some(id))
            .find_map(|c| releases::repo_of(&c.entry.homepage))
    }

    /// The list's lines, a line a copy.
    pub fn lines(&self) -> Vec<LineData> {
        let mut seen: Vec<&str> = Vec::new();
        self.copies
            .iter()
            .map(|c| {
                let id = c.entry.id.as_deref();
                let first = id.is_some_and(|id| {
                    let first = !seen.contains(&id);
                    seen.push(id);
                    first
                });
                let update = match id {
                    Some(id) if first => self
                        .update_for(id)
                        .map(|r| r.version.to_string())
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                LineData {
                    name: if c.network {
                        format!("! {}", c.entry.name)
                    } else {
                        c.entry.name.clone()
                    },
                    version: c
                        .entry
                        .version
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    path: c.entry.path.display().to_string(),
                    removable: c.entry.deletable,
                    update,
                }
            })
            .collect()
    }

    /// Puts a copy in the list beside the plugin's others, or at the end;
    /// one at the same path is replaced.
    fn put(&mut self, copy: Copy) {
        if let Some(c) = self
            .copies
            .iter_mut()
            .find(|c| c.entry.path == copy.entry.path)
        {
            *c = copy;
            return;
        }
        let at = copy.entry.id.as_deref().and_then(|id| {
            self.copies
                .iter()
                .rposition(|c| c.entry.id.as_deref() == Some(id))
                .map(|i| i + 1)
        });
        match at {
            Some(i) => self.copies.insert(i, copy),
            None => self.copies.push(copy),
        }
    }
}

/// The copies in list order: those of a plugin together, plugins in the
/// order OxiGuitar first found them.
pub fn grouped(copies: Vec<Copy>) -> Vec<Copy> {
    let mut order: Vec<Option<String>> = Vec::new();
    for c in &copies {
        if c.entry.id.is_none() || !order.contains(&c.entry.id) {
            order.push(c.entry.id.clone());
        }
    }
    let mut out = Vec::with_capacity(copies.len());
    let mut rest = copies;
    for id in order {
        match id {
            Some(id) => {
                let (mine, others): (Vec<Copy>, Vec<Copy>) = rest
                    .into_iter()
                    .partition(|c| c.entry.id.as_deref() == Some(id.as_str()));
                out.extend(mine);
                rest = others;
            }
            None => {
                if let Some(i) = rest.iter().position(|c| c.entry.id.is_none()) {
                    out.push(rest.remove(i));
                }
            }
        }
    }
    out
}

/// What the plugin, its window and the work off their threads share.
#[derive(Debug)]
pub struct Shared {
    host: Host,
    /// Asks GitHub, keeping answers for the session.
    pub checker: Checker,
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

    /// The state.
    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Lists what OxiGuitar found, for a new opening of the window; returns
    /// the opening's number, the files to look at and the repositories to
    /// ask.
    pub fn open(&self) -> Result<(u64, Vec<PathBuf>, Vec<Repo>), Error> {
        let found = self.host.plugin_files()?;
        let user_folder = PathBuf::from(&found.user_folder);
        install::clean(&user_folder);
        let copies = grouped(
            found
                .files
                .into_iter()
                .map(|f| Copy {
                    entry: Entry::of(f),
                    network: false,
                })
                .collect(),
        );
        let mut repos: Vec<Repo> = copies
            .iter()
            .filter(|c| c.entry.id.is_some())
            .filter_map(|c| releases::repo_of(&c.entry.homepage))
            .collect();
        repos.sort_by_key(ToString::to_string);
        repos.dedup();
        let paths = copies.iter().map(|c| c.entry.path.clone()).collect();
        let mut state = self.state();
        let generation = state.generation + 1;
        *state = State {
            generation,
            user_folder,
            copies,
            ..State::default()
        };
        Ok((generation, paths, repos))
    }

    /// Changes the state if the window is still the one opened as
    /// `generation`; whether it was.
    pub fn change(&self, generation: u64, change: impl FnOnce(&mut State)) -> bool {
        let mut state = self.state();
        if state.generation != generation {
            return false;
        }
        change(&mut state);
        true
    }

    /// The window was closed: late answers are dropped.
    pub fn closed(&self) {
        let mut state = self.state();
        state.generation += 1;
        state.busy = false;
    }

    /// Off the window's thread: whether each file can reach the network.
    pub fn look_at_files(&self, generation: u64, paths: &[PathBuf]) -> bool {
        let marks: Vec<(PathBuf, bool)> = paths
            .iter()
            .map(|p| (p.clone(), network::file_uses_network(p).unwrap_or(false)))
            .collect();
        self.change(generation, |s| {
            for (path, mark) in marks {
                if let Some(c) = s.copies.iter_mut().find(|c| c.entry.path == path) {
                    c.network = mark;
                }
            }
        })
    }

    /// Off the window's thread: `repo`'s newest release.
    pub fn ask(&self, generation: u64, repo: &Repo) -> bool {
        let found = self.checker.check(repo);
        self.change(generation, |s| {
            if let Found::Failed(why) = &found
                && s.status.is_empty()
            {
                s.status = format!("Can't check for updates: {why}.");
            }
            s.latest.insert(repo.clone(), found);
        })
    }

    /// Copies the plugin file `file` into the user's folder.
    pub fn add(&self, file: &Path) {
        let folder = self.state().user_folder.clone();
        let result = install::install_file(file, &folder);
        let mut s = self.state();
        match result {
            Ok((to, manifest)) => {
                let network = network::file_uses_network(&to).unwrap_or(false);
                s.status = format!(
                    "Added {} {}. OxiGuitar uses it when it starts again.",
                    manifest.name, manifest.version
                );
                s.put(Copy {
                    entry: added(&to, &manifest, PluginFolderKind::User),
                    network,
                });
            }
            Err(why) => {
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                s.status = format!("{name} can't be added: {why}.");
            }
        }
    }

    /// Deletes the copy at `path`, once the user confirmed it.
    pub fn delete(&self, path: &Path) {
        let mut s = self.state();
        let Some(i) = s.copies.iter().position(|c| c.entry.path == path) else {
            return;
        };
        if !s.copies[i].entry.deletable {
            return;
        }
        let name = s.copies[i].entry.name.clone();
        match install::delete(path) {
            Ok(()) => {
                s.copies.remove(i);
                s.status =
                    format!("Deleted {name}. OxiGuitar stops using it when it starts again.");
            }
            Err(e) => s.status = format!("{name} can't be deleted: {e}."),
        }
    }

    /// Marks an update of the plugin at line `i` as started; the plugin's
    /// id and the release, unless there is none or one is running.
    pub fn start_update(&self, i: usize) -> Option<(u64, String, Release)> {
        let mut s = self.state();
        if s.busy {
            return None;
        }
        let id = s.copies.get(i)?.entry.id.clone()?;
        let release = s.update_for(&id)?;
        s.busy = true;
        s.status = String::new();
        Some((s.generation, id, release))
    }

    /// Off the window's thread: puts `release` of `id` in the folder of the
    /// copy OxiGuitar uses when it may be written there, or else in the
    /// user's folder; that folder's older copies go.
    pub fn update(&self, generation: u64, id: &str, release: &Release) {
        let (folder, kept, kind) = {
            let s = self.state();
            let (folder, kept) = update_folder(&s.copies, id, &s.user_folder);
            // The folder's kind, from a copy OxiGuitar found there.
            let kind = s
                .copies
                .iter()
                .find(|c| c.entry.path.parent() == Some(folder.as_path()))
                .map_or(PluginFolderKind::User, |c| c.entry.folder);
            (folder, kept, kind)
        };
        let user_folder = folder;
        let result = (|| -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
            let zip = self.checker.download(&release.zip)?;
            if let Some(sums) = &release.sums {
                let sums = self.checker.download(sums)?;
                install::check_sum(&zip, &release.zip_name, &String::from_utf8_lossy(&sums))?;
            }
            let files =
                install::unpack(&zip, id, &release.version.to_string(), releases::target())?;
            std::fs::create_dir_all(&user_folder).map_err(|e| e.to_string())?;
            let mut written = Vec::new();
            for (name, data) in files {
                let to = user_folder.join(&name);
                install::replace(&to, &data)
                    .map_err(|e| format!("{name} can't be written to your plugin folder ({e})"))?;
                written.push((to, data));
            }
            Ok(written)
        })();
        self.change(generation, |s| {
            s.busy = false;
            let name = s
                .copies
                .iter()
                .find(|c| c.entry.id.as_deref() == Some(id))
                .map_or_else(|| id.to_owned(), |c| c.entry.name.clone());
            match result {
                Ok(written) => {
                    let paths: Vec<&PathBuf> = written.iter().map(|(p, _)| p).collect();
                    // The user's older copies go: the new one is used anyway.
                    let older: Vec<PathBuf> = s
                        .copies
                        .iter()
                        .filter(|c| {
                            c.entry.id.as_deref() == Some(id)
                                && c.entry.path.parent() == Some(user_folder.as_path())
                                && !paths.contains(&&c.entry.path)
                        })
                        .map(|c| c.entry.path.clone())
                        .collect();
                    for path in older {
                        if install::delete(&path).is_ok() {
                            s.copies.retain(|c| c.entry.path != path);
                        }
                    }
                    for (to, data) in &written {
                        if let Ok(manifest) = install::manifest_of(data) {
                            let network = network::uses_network(data).unwrap_or(false);
                            s.put(Copy {
                                entry: added(to, &manifest, kind),
                                network,
                            });
                        }
                    }
                    s.status = match &kept {
                        None => format!(
                            "Updated {name} to {}. OxiGuitar uses it when it starts again.",
                            release.version
                        ),
                        Some(used) => format!(
                            "Put {name} {} in your plugin folder, but OxiGuitar keeps using {}, \
                             which comes with it.",
                            release.version,
                            used.display()
                        ),
                    };
                }
                Err(why) => s.status = format!("{name} can't be updated: {why}."),
            }
        });
    }
}

/// Where an update of `id` goes: the folder of the copy OxiGuitar uses when
/// it may be written there, or else `user_folder`; with the copy that
/// stays in use, when it is one beside the program that can't be replaced
/// (an AppImage's or the macOS app's), which comes before the user's.
pub fn update_folder(copies: &[Copy], id: &str, user_folder: &Path) -> (PathBuf, Option<PathBuf>) {
    let used = copies
        .iter()
        .find(|c| c.entry.id.as_deref() == Some(id) && c.entry.used());
    match used {
        Some(c) if c.entry.deletable => (
            c.entry
                .path
                .parent()
                .map_or_else(|| user_folder.to_path_buf(), Path::to_path_buf),
            None,
        ),
        Some(c) if c.entry.folder == PluginFolderKind::Program => {
            (user_folder.to_path_buf(), Some(c.entry.path.clone()))
        }
        _ => (user_folder.to_path_buf(), None),
    }
}

/// The list's entry for a file put in a folder of `folder`'s kind.
fn added(
    path: &Path,
    manifest: &oxiguitar_plugin::api::manifest::Manifest,
    folder: PluginFolderKind,
) -> Entry {
    Entry {
        path: path.to_path_buf(),
        folder,
        id: Some(manifest.id.clone()),
        name: manifest.name.clone(),
        version: semver::Version::parse(&manifest.version.to_string()).ok(),
        homepage: manifest.homepage.clone(),
        state: PluginFileState::Used,
        deletable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxiguitar_plugin::api::message::{HostAnswer, HostRequest};
    use oxiguitar_plugin::{HostServices, PluginFile, PluginFiles};
    use std::sync::Arc;

    struct Listed(PluginFiles);

    impl HostServices for Listed {
        fn request(&self, request: HostRequest<'_>) -> Result<HostAnswer<'static>, Error> {
            match request {
                HostRequest::PluginFiles => Ok(HostAnswer::PluginFiles(self.0.clone())),
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
            // Not GitHub: the tests ask nobody.
            homepage: "https://example.org".into(),
            state: PluginFileState::Used,
            loaded: false,
        }
    }

    #[test]
    fn an_update_goes_where_the_used_copy_is_when_it_can() {
        let dir = std::env::temp_dir().join(format!("pm-update-{}", std::process::id()));
        let user = dir.join("user");
        let portable = dir.join("portable");
        std::fs::create_dir_all(&portable).unwrap();
        let copy = |path: PathBuf, folder| Copy {
            entry: Entry::of(file(&path, folder, "a.b", "0.1.0")),
            network: false,
        };
        // In the user's folder.
        let copies = [copy(user.join("a.oxp"), PluginFolderKind::User)];
        assert_eq!(update_folder(&copies, "a.b", &user), (user.clone(), None));
        // Beside a portable program, whose folder can be written: there.
        let copies = [copy(portable.join("a.oxp"), PluginFolderKind::Program)];
        assert_eq!(
            update_folder(&copies, "a.b", &user),
            (portable.clone(), None)
        );
        // Beside an AppImage's program, which can't be written: the
        // user's folder, and that copy stays in use.
        let appimage = dir.join("mounted").join("a.oxp");
        let copies = [copy(appimage.clone(), PluginFolderKind::Program)];
        assert_eq!(
            update_folder(&copies, "a.b", &user),
            (user.clone(), Some(appimage))
        );
        // A package's: the user's folder, which comes after it.
        let copies = [copy(
            dir.join("system").join("a.oxp"),
            PluginFolderKind::System,
        )];
        assert_eq!(update_folder(&copies, "a.b", &user), (user.clone(), None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn release(v: &str) -> Release {
        Release {
            version: semver::Version::parse(v).unwrap(),
            zip_name: format!("a-{v}-linux-x86_64.zip"),
            zip: "https://github.com/a/b/z.zip".into(),
            sums: None,
        }
    }

    #[test]
    fn a_plugin_s_copies_are_listed_together_and_system_ones_stay() {
        let dir = std::env::temp_dir().join(format!("oxiguitar-pm-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let user = dir.join("plugins");
        std::fs::create_dir_all(&user).unwrap();
        let mine = user.join("a.oxp");
        std::fs::write(&mine, b"not a library").unwrap();
        let system = Path::new("/usr/lib/oxiguitar/plugins");
        let shared = Arc::new(Shared::new(Host::new(Arc::new(Listed(PluginFiles {
            user_folder: user.to_str().unwrap().into(),
            files: vec![
                PluginFile {
                    state: PluginFileState::Replaced(mine.to_str().unwrap().into()),
                    ..file(
                        &system.join("a.oxp"),
                        PluginFolderKind::System,
                        "a.b",
                        "0.1.0",
                    )
                },
                file(
                    &system.join("c.oxp"),
                    PluginFolderKind::System,
                    "c.d",
                    "1.0.0",
                ),
                file(&mine, PluginFolderKind::User, "a.b", "0.2.0"),
            ],
        })))));
        let (generation, paths, repos) = shared.open().unwrap();
        assert_eq!(paths.len(), 3);
        assert!(repos.is_empty());
        let names = |s: &State| {
            s.lines()
                .iter()
                .map(|l| (l.name.clone(), l.removable))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&shared.state()),
            [
                ("a.b".to_owned(), false),
                ("a.b".to_owned(), true),
                ("c.d".to_owned(), false)
            ],
            "a plugin's copies together; only the user's can go"
        );
        assert!(shared.look_at_files(generation, &paths));
        // A system copy stays whatever is asked; the user's goes.
        shared.delete(&system.join("a.oxp"));
        assert_eq!(shared.state().copies.len(), 3);
        shared.delete(&mine);
        assert!(!mine.exists());
        assert_eq!(shared.state().copies.len(), 2);
        assert!(shared.state().status.starts_with("Deleted a.b."));
        // An earlier opening's answers are dropped.
        shared.closed();
        assert!(!shared.change(generation, |_| {}));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_update_is_offered_once_a_plugin_when_a_release_is_newer() {
        let repo = Repo {
            owner: "a".into(),
            name: "b".into(),
        };
        let copy = |path: &str, version: &str| Copy {
            entry: Entry::of(PluginFile {
                homepage: "https://github.com/a/b".into(),
                ..file(Path::new(path), PluginFolderKind::System, "a.b", version)
            }),
            network: true,
        };
        let mut s = State {
            generation: 1,
            copies: vec![copy("/x/a.oxp", "0.2.0"), copy("/y/a.oxp", "0.1.0")],
            ..State::default()
        };
        assert_eq!(s.lines()[0].name, "! a.b");
        assert_eq!(s.lines()[0].update, "", "not asked yet");
        s.latest
            .insert(repo.clone(), Found::Release(release("0.2.0")));
        assert!(s.update_for("a.b").is_none(), "up to date");
        s.latest
            .insert(repo.clone(), Found::Release(release("0.3.0-beta.1")));
        let lines = s.lines();
        assert_eq!(lines[0].update, "0.3.0-beta.1");
        assert_eq!(lines[1].update, "", "on the plugin's first line only");
        assert_eq!(lines[0].version, "0.2.0");
        s.latest.insert(repo, Found::Nothing);
        assert!(s.update_for("a.b").is_none());
    }

    #[test]
    fn added_copies_join_their_plugin_and_replace_one_at_their_path() {
        let entry = |path: &str, id: &str| Copy {
            entry: Entry::of(file(Path::new(path), PluginFolderKind::System, id, "1.0.0")),
            network: false,
        };
        let mut s = State {
            copies: grouped(vec![
                entry("/s/a.oxp", "a.a"),
                entry("/s/b.oxp", "b.b"),
                entry("/u/a.oxp", "a.a"),
            ]),
            ..State::default()
        };
        let paths = |s: &State| {
            s.copies
                .iter()
                .map(|c| c.entry.path.display().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(paths(&s), ["/s/a.oxp", "/u/a.oxp", "/s/b.oxp"]);
        s.put(entry("/u/b.oxp", "b.b"));
        s.put(entry("/u/c.oxp", "c.c"));
        s.put(entry("/u/a.oxp", "a.a"));
        assert_eq!(
            paths(&s),
            ["/s/a.oxp", "/u/a.oxp", "/s/b.oxp", "/u/b.oxp", "/u/c.oxp"]
        );
    }
}

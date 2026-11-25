// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The Plugin Manager's UI (plugin interface 1.15; mateusrdp/oxiguitar#346):
//! `ui/manager.slint`, which OxiGuitar compiles and shows with its own
//! Slint, widgets and style. The plugin draws nothing and carries no widget
//! code: it sets the views' properties and hears their callbacks.
//!
//! Two views, both windows of their own: `manager`, the list with Add…,
//! Update, the red X and Close; and `confirm`, the question before the X
//! deletes a file. Work that waits (looking at files, asking GitHub,
//! downloading, the file dialog) runs on threads of the plugin's own, and
//! sets the properties again when done.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use oxiguitar_plugin::{Error, Host, TrackView, Value};

use crate::manager::{LineData, Shared};

/// The list's window.
pub const MANAGER: &str = "manager";

/// The question before a file is deleted.
pub const CONFIRM: &str = "confirm";

/// The Plugin Manager's UI.
#[derive(Debug)]
pub struct ManagerUi {
    host: Host,
    shared: Arc<Shared>,
    /// The file the question is about.
    asking: Mutex<Option<PathBuf>>,
}

impl ManagerUi {
    /// Nothing shown yet.
    pub fn new(host: Host, shared: Arc<Shared>) -> ManagerUi {
        ManagerUi {
            host,
            shared,
            asking: Mutex::new(None),
        }
    }

    /// Opens the list on what OxiGuitar found, then looks at the files and
    /// asks GitHub, setting the list again as answers come. What to say in
    /// the footer when it can't.
    pub fn open(self: &Arc<Self>) -> String {
        let (generation, paths, repos) = match self.shared.open() {
            Ok(opened) => opened,
            Err(e) => return format!("The plugin files can't be listed: {}.", e.message),
        };
        self.push();
        if let Err(e) = self.host.show_view(MANAGER) {
            return format!("The Plugin Manager can't open: {}.", e.message);
        }
        let ui = self.clone();
        let _ = std::thread::Builder::new()
            .name("plugin-manager-look".into())
            .spawn(move || {
                if ui.shared.look_at_files(generation, &paths) {
                    ui.push();
                }
                std::thread::scope(|scope| {
                    for repo in &repos {
                        let ui = &ui;
                        scope.spawn(move || {
                            if ui.shared.ask(generation, repo) {
                                ui.push();
                            }
                        });
                    }
                });
            });
        String::new()
    }

    /// Sets the list's properties from the state.
    fn push(&self) {
        let (lines, status, busy) = {
            let s = self.shared.state();
            (s.lines(), s.status.clone(), s.busy)
        };
        let _ = self.host.set_view(
            MANAGER,
            vec![
                (
                    "lines".into(),
                    Value::List(lines.iter().map(line_value).collect()),
                ),
                ("status".into(), Value::String(status)),
                ("busy".into(), Value::Bool(busy)),
            ],
        );
    }

    /// The system's file dialog (the plugin's own, through the desktop
    /// portal on Linux), off OxiGuitar's threads; the chosen file is
    /// copied into the user's folder.
    fn add(self: &Arc<Self>) {
        let ui = self.clone();
        let _ = std::thread::Builder::new()
            .name("plugin-manager-add".into())
            .spawn(move || {
                let Some(file) = rfd::FileDialog::new()
                    .set_title("Add a Plugin File")
                    .add_filter("OxiGuitar plugins", &["oxp"])
                    .pick_file()
                else {
                    return;
                };
                ui.shared.add(&file);
                ui.push();
            });
    }

    /// The X of line `i`: the question first.
    fn ask_to_remove(&self, i: usize) -> Result<(), Error> {
        let Some(copy) = self.shared.state().copies.get(i).cloned() else {
            return Ok(());
        };
        if !copy.entry.deletable {
            return Ok(());
        }
        self.host.set_view(
            CONFIRM,
            vec![
                ("name".into(), Value::String(copy.entry.name.clone())),
                (
                    "path".into(),
                    Value::String(copy.entry.path.display().to_string()),
                ),
            ],
        )?;
        *self.asking.lock().unwrap_or_else(PoisonError::into_inner) = Some(copy.entry.path);
        self.host.show_view(CONFIRM)
    }

    /// The question's answer.
    fn answered(&self, delete: bool) -> Result<(), Error> {
        let path = self
            .asking
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        self.host.close_view(CONFIRM)?;
        if let Some(path) = path.filter(|_| delete) {
            self.shared.delete(&path);
            self.push();
        }
        Ok(())
    }

    /// Update on line `i`: the newest release goes in, off OxiGuitar's
    /// threads.
    fn update(self: &Arc<Self>, i: usize) {
        let Some((generation, id, release)) = self.shared.start_update(i) else {
            return;
        };
        self.push();
        let ui = self.clone();
        let _ = std::thread::Builder::new()
            .name("plugin-manager-update".into())
            .spawn(move || {
                ui.shared.update(generation, &id, &release);
                ui.push();
            });
    }
}

/// A line of the list as the window's `Line` struct.
fn line_value(line: &LineData) -> Value {
    Value::Struct(vec![
        ("name".into(), Value::String(line.name.clone())),
        ("path".into(), Value::String(line.path.clone())),
        ("removable".into(), Value::Bool(line.removable)),
        ("update".into(), Value::String(line.update.clone())),
        ("version".into(), Value::String(line.version.clone())),
    ])
}

/// The index a callback names, as Slint passes an `int`.
fn index(args: &[Value]) -> Option<usize> {
    match args.first() {
        Some(Value::Number(n)) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as usize),
        _ => None,
    }
}

/// What the plugin hands OxiGuitar: the views and what their callbacks do.
#[derive(Debug)]
pub struct Ui(pub Arc<ManagerUi>);

/// The `.slint` file is `ui/manager.slint` in the plugin file, which the
/// manifest names (interface 1.16): OxiGuitar compiles it from there.
impl oxiguitar_plugin::Ui for Ui {
    fn callback(&self, view: &str, callback: &str, args: Vec<Value>) -> Result<(), Error> {
        let ui = &self.0;
        match (view, callback) {
            (MANAGER, "add") => ui.add(),
            (MANAGER, "remove") => {
                if let Some(i) = index(&args) {
                    ui.ask_to_remove(i)?;
                }
            }
            (MANAGER, "update") => {
                if let Some(i) = index(&args) {
                    ui.update(i);
                }
            }
            (MANAGER, "close") => {
                ui.host.close_view(MANAGER)?;
            }
            (CONFIRM, "confirmed") => ui.answered(true)?,
            (CONFIRM, "cancelled") => ui.answered(false)?,
            _ => {}
        }
        Ok(())
    }

    fn shown(&self, _view: &str, _track: Option<&TrackView>) -> Result<(), Error> {
        Ok(())
    }

    fn closed(&self, view: &str, _accepted: bool) -> Result<(), Error> {
        match view {
            // The list closed: late answers are dropped, and the question
            // goes with it.
            MANAGER => {
                self.0.shared.closed();
                let _ = self.0.host.close_view(CONFIRM);
            }
            CONFIRM => {
                self.0
                    .asking
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_the_window_s_struct_and_an_index_is_a_whole_number() {
        let line = LineData {
            name: "! Plugin Manager".into(),
            version: "0.1.0".into(),
            path: "/home/u/plugins/plugin-manager.oxp".into(),
            removable: true,
            update: String::new(),
        };
        let Value::Struct(fields) = line_value(&line) else {
            panic!("a struct");
        };
        let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["name", "path", "removable", "update", "version"]);
        assert_eq!(index(&[Value::Number(3.0)]), Some(3));
        assert_eq!(index(&[Value::Number(-1.0)]), None);
        assert_eq!(index(&[Value::Number(1.5)]), None);
        assert_eq!(index(&[]), None);
    }

    #[test]
    fn the_file_has_both_views_with_what_the_plugin_sets_and_hears() {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/manager.slint");
        let compiled = oxiguitar_plugin_test::compile_ui_file(&file).unwrap();
        let manager = compiled.component("ManagerWindow").unwrap();
        assert_eq!(manager.properties, ["busy", "lines", "status"]);
        assert_eq!(manager.callbacks, ["add", "close", "remove", "update"]);
        let confirm = compiled.component("ConfirmDialog").unwrap();
        assert_eq!(confirm.properties, ["name", "path"]);
        assert_eq!(confirm.callbacks, ["cancelled", "confirmed"]);
    }
}

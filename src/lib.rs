// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! A plugin for OxiGuitar that manages the others (plugin interface 1.15;
//! mateusrdp/oxiguitar#344). Plugins → Plugin Manager… opens its dialog:
//! every plugin file OxiGuitar found, every copy of a plugin included, with
//! its version, its folder, whether OxiGuitar uses it, whether it can reach
//! the network, and the newest release; from there the user updates a
//! plugin, installs a plugin file they downloaded, or deletes a copy.
//!
//! - `dialog` is what the user sees, and does what they ask.
//! - `files` turns what OxiGuitar found (`Host::plugin_files`) into the
//!   dialog's rows: what each copy is, and whether it may be deleted (never
//!   one in a system folder).
//! - `network` tells from a plugin file's imported functions whether it can
//!   reach the network: no manifest says so (Mateus, 24 Nov 2026: authors
//!   wouldn't fill it in and could say anything), and the import table
//!   reads in milliseconds.
//! - `releases` asks GitHub for the newest release of each plugin whose
//!   homepage is a GitHub repository, each time the dialog opens.
//! - `install` puts files in the user's plugin folder: a file the user
//!   chose, or a release's file for this system.
//!
//! Unlike OxiGuitar and its other plugins, this plugin goes to the network
//! (github.com) every time its dialog opens, and only then. It sends
//! nothing about the user: each request names a repository. It is not one
//! of the plugins OxiGuitar suggests (#344): no package recommends it, and
//! OxiGuitar's manual doesn't list it.

use std::sync::Arc;

use oxiguitar_plugin::{Error, Host, Plugin, Tab, Tool};

pub mod dialog;
pub mod files;
pub mod install;
pub mod network;
pub mod releases;

/// The plugin's id.
pub const ID: &str = "oxiguitar.plugin-manager";

/// The tool's id in the manifest.
pub const TOOL: &str = "manage";

/// The plugin.
#[derive(Debug)]
pub struct ManagerPlugin {
    tool: dialog::ManageTool,
    dialog: dialog::Dialog,
}

impl Plugin for ManagerPlugin {
    fn start(host: Host) -> Result<ManagerPlugin, Error> {
        let shared = Arc::new(dialog::Shared::new(host));
        Ok(ManagerPlugin {
            tool: dialog::ManageTool {
                shared: shared.clone(),
            },
            dialog: dialog::Dialog { shared },
        })
    }

    fn tool(&self, id: &str) -> Option<&dyn Tool> {
        (id == TOOL).then_some(&self.tool as &dyn Tool)
    }

    fn tab(&self, id: &str) -> Option<&dyn Tab> {
        (id == dialog::DIALOG).then_some(&self.dialog as &dyn Tab)
    }
}

oxiguitar_plugin::export_plugin!(ManagerPlugin);

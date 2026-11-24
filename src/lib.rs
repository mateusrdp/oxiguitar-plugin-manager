// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! A plugin for OxiGuitar that manages the others (plugin interface 1.15;
//! mateusrdp/oxiguitar#344). Plugins → Plugin Manager… opens its own window:
//! every plugin file OxiGuitar found, every copy of a plugin included, with
//! its name (after `!` when it can reach the network), version and path;
//! Add… copies a plugin file into the user's folder, Update puts in the
//! newest release, and the red X deletes a copy outside the system's
//! folders, once confirmed.
//!
//! - `ui` is its UI: `ui/manager.slint`, which OxiGuitar compiles and shows
//!   with its own Slint (#346); the plugin sets properties and hears
//!   callbacks, and draws nothing itself.
//! - `manager` is what the window shows and what its buttons do.
//! - `files` turns what OxiGuitar found into the list's entries, and
//!   whether each may be deleted (never one in a system folder).
//! - `network` tells from a plugin file's imported functions whether it can
//!   reach the network: no manifest says so (Mateus, 24 Nov 2026: authors
//!   wouldn't fill it in and could say anything), and the import table
//!   reads in milliseconds.
//! - `releases` asks GitHub for the newest release of each plugin whose
//!   homepage is a GitHub repository, each time the window opens.
//! - `install` puts files in the user's plugin folder: a file the user
//!   chose, or a release's file for this system.
//!
//! The plugin goes to the network (github.com) each time its window opens,
//! and when the user updates a plugin; each request names a repository and
//! nothing else.

use std::sync::Arc;

use oxiguitar_plugin::{Error, Host, Plugin, Tool};

pub mod files;
pub mod install;
pub mod manager;
pub mod network;
pub mod releases;
pub mod ui;

/// The plugin's id.
pub const ID: &str = "oxiguitar.plugin-manager";

/// The tool's id in the manifest.
pub const TOOL: &str = "manage";

/// The plugin.
#[derive(Debug)]
pub struct ManagerPlugin {
    tool: ManageTool,
    ui: ui::Ui,
}

/// Plugins → Plugin Manager…: opens the window.
#[derive(Debug)]
struct ManageTool(Arc<ui::ManagerUi>);

impl Tool for ManageTool {
    fn run(&self) -> Result<String, Error> {
        Ok(self.0.open())
    }
}

impl Plugin for ManagerPlugin {
    fn start(host: Host) -> Result<ManagerPlugin, Error> {
        let shared = Arc::new(manager::Shared::new(host.clone()));
        let ui = Arc::new(ui::ManagerUi::new(host, shared));
        Ok(ManagerPlugin {
            tool: ManageTool(ui.clone()),
            ui: ui::Ui(ui),
        })
    }

    fn tool(&self, id: &str) -> Option<&dyn Tool> {
        (id == TOOL).then_some(&self.tool as &dyn Tool)
    }

    fn ui(&self) -> Option<&dyn oxiguitar_plugin::Ui> {
        Some(&self.ui)
    }
}

oxiguitar_plugin::export_plugin!(ManagerPlugin);

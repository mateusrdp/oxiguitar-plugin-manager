// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The contract suite: builds this crate as an `.oxp` file and checks it
//! through OxiGuitar's own plugin host, as the app would load it. Then the
//! file itself: its manifest reads as the list reads other plugins', and
//! it is marked as reaching the network, which it does. `cargo test` runs
//! it with the rest.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use oxiguitar_host::{Plugins, SharedSettings, Unheard};

/// This plugin as the `.oxp` file we ship (a debug build), built once.
fn ours() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let crate_folder = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Builds target/oxp/plugin-manager.oxp: the package's name, then
        // the file's.
        oxiguitar_plugin_test::build(
            crate_folder,
            env!("CARGO_PKG_NAME"),
            "plugin-manager",
            false,
        )
        .expect("the .oxp file builds")
    })
}

#[test]
fn the_plugin_file_keeps_the_contract() {
    let report = oxiguitar_plugin_test::check(ours());
    for passed in &report.passed {
        println!("ok    {passed}");
    }
    assert!(report.ok(), "failed: {:#?}", report.failed);
}

#[test]
fn the_plugin_file_is_read_as_the_list_reads_others() {
    let data = std::fs::read(ours()).unwrap();
    let manifest = oxiguitar_plugin_manager::install::manifest_of(&data).unwrap();
    assert_eq!(manifest.id, oxiguitar_plugin_manager::ID);
    assert!(
        oxiguitar_plugin_manager::network::uses_network(&data).unwrap(),
        "it asks GitHub, so it is marked"
    );
}

/// OxiGuitar lists the plugin among what it found, as the window will be
/// told, and installing it in a user's folder makes a copy OxiGuitar uses.
#[test]
fn a_plugin_file_installed_in_the_user_s_folder_is_found_there() {
    let scratch =
        std::env::temp_dir().join(format!("oxiguitar-pm-installed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let user = scratch.join("plugins");
    let (to, manifest) = oxiguitar_plugin_manager::install::install_file(ours(), &user).unwrap();
    assert_eq!(to, user.join("plugin-manager.oxp"));
    assert_eq!(manifest.id, oxiguitar_plugin_manager::ID);
    let plugins = Plugins::discover(
        std::slice::from_ref(&user),
        SharedSettings::default(),
        oxiguitar_host::NO_DISPLAY,
        Arc::new(Unheard),
    );
    let found = plugins.found();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, to);
    assert_eq!(
        found[0].manifest.as_ref().map(|m| m.id.as_str()),
        Some(oxiguitar_plugin_manager::ID)
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

# Changelog

All notable changes to the Plugin Manager. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-25

### Changed

- **Plugin files are archives** (mateusrdp/oxiguitar#347, plugin interface 1.16). The Plugin Manager reads a plugin file's manifest and library from the archive without unpacking it, and says when a file is of the format before OxiGuitar 0.23. Its own file is packed that way (`build-oxp.sh`), with `ui/manager.slint` beside the library, which the manifest names (`ui`). It needs the OxiGuitar after 0.22.0; the SDK is at OxiGuitar v0.23.0.
- **Update goes where OxiGuitar looks**: OxiGuitar now uses the copy in the folder it searches last, whatever the versions (beside the program, then yours, then the system's), so an update is written where the copy in use is when the plugin may write there (your folder, or beside a portable copy), and into your folder otherwise; when a copy inside an AppImage or the macOS app stays in use, the window says so.
- `scripts/test-all.py` 1.1.0, as in every repository.

## [0.1.0] - 2026-11-24

### Added

- **The Plugin Manager** (mateusrdp/oxiguitar#344): Plugins → Plugin Manager… opens its window, **Add…** at the top, **Close** at the bottom, and in between every plugin file OxiGuitar found, every copy of a plugin on its own line with its name, version and path. A `!` before a name marks a plugin that can reach the network, told from the system functions its file imports. Each time it opens it asks GitHub for the newest release of every plugin whose homepage is a GitHub repository (pre-releases included) and offers **Update to …** when one is newer than every copy: the release's file for this system, checked against its SHA256SUMS, goes into your plugin folder and replaces your older copies there. **Add…** copies a `.oxp` you choose there; the **red X** deletes a copy after asking, and is greyed out for a copy in a system folder. Changes are used when OxiGuitar starts again. The window is a Slint file OxiGuitar shows with its own widgets and style (plugin interface 1.15, mateusrdp/oxiguitar#346).

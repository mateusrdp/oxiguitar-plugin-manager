# Changelog

All notable changes to the Plugin Manager. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **The Plugin Manager** (mateusrdp/oxiguitar#344): Plugins → Plugin Manager… opens its window, **Add…** at the top, **Close** at the bottom, and in between every plugin file OxiGuitar found, every copy of a plugin on its own line with its name, version and path. A `!` before a name marks a plugin that can reach the network, told from the system functions its file imports. Each time it opens it asks GitHub for the newest release of every plugin whose homepage is a GitHub repository (pre-releases included) and offers **Update to …** when one is newer than every copy: the release's file for this system, checked against its SHA256SUMS, goes into your plugin folder and replaces your older copies there. **Add…** copies a `.oxp` you choose there; the **red X** deletes a copy after asking, and is greyed out for a copy in a system folder. Changes are used when OxiGuitar starts again. The window is a Slint file OxiGuitar shows with its own widgets and style (plugin interface 1.15, mateusrdp/oxiguitar#346).

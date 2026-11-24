# Changelog

All notable changes to the Plugin Manager. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **The Plugin Manager** (mateusrdp/oxiguitar#344): Plugins → Plugin Manager… lists every plugin file OxiGuitar found, every copy of a plugin on its own line, with its version, its folder and whether OxiGuitar uses it. A `!` marks a plugin that can reach the network, told from the system functions its file imports. Each time it opens it asks GitHub for the newest release of every plugin whose homepage is a GitHub repository (pre-releases included) and offers **Update** when one is newer than every copy: the release's file for this system, checked against its SHA256SUMS, goes into your plugin folder and replaces your older copies. **Install a Plugin File…** copies a `.oxp` you choose there; **Delete** removes a copy, except in a system folder. Changes are used when OxiGuitar starts again. Needs OxiGuitar with plugin interface 1.15.

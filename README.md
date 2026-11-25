# oxiguitar-plugin-manager

A plugin for [OxiGuitar](https://github.com/mateusrdp/oxiguitar) that manages
the others: every copy of every plugin you have, with its version and the
newest release, and buttons to update a plugin, add a plugin file you
downloaded, or delete a copy.

In plain words: OxiGuitar's own Tools → Plugins lists your plugins and
switches them on and off, and never touches a file. This plugin does the
rest, in your own plugin folder. **Each time you open it, it asks github.com
for every plugin's newest release**, and it downloads one when you press
Update. Each question names a plugin's repository and nothing else.

- **Plugins → Plugin Manager…** opens its window: **Add…** at the top, the
  plugin files OxiGuitar found when it started in a list, **Close** at the
  bottom.
- **Each line** is one copy of a plugin: its name and version, and its file's
  path. A `!` before the name marks a plugin that can reach the network, told
  from the system functions the file uses (this plugin is marked too).
- **Update to …** appears beside a plugin when its newest release on GitHub is
  newer than every copy you have. The release's file for your system is
  checked against its SHA256SUMS and goes into your plugin folder, replacing
  your older copies there.
- **Add…** copies a `.oxp` file you choose into your plugin folder.
- **The red X** deletes a copy, after asking. A copy in a system folder (where
  packages install it) can't be deleted here: its X is greyed out.

OxiGuitar reads its plugin folders when it starts, so changes are used the
next time you start it.

The window is a Slint file OxiGuitar shows with its own widgets and style
(plugin interface 1.16, its file packed in the plugin's `.oxp`); the
plugin draws nothing itself.

## Manual

The [manual](https://mateusrdp.github.io/oxiguitar-plugin-manager/) says how
to install it and use it, how it works, and how it is tested.

## Credits

Built on OxiGuitar's plugin SDK, with [ureq](https://github.com/algesten/ureq),
[rustls](https://github.com/rustls/rustls) and
[ring](https://github.com/briansmith/ring) for HTTPS,
[object](https://github.com/gimli-rs/object) to read plugin files,
[zip](https://github.com/zip-rs/zip2), [RustCrypto's sha2](https://github.com/RustCrypto/hashes),
[semver](https://github.com/dtolnay/semver) and
[rfd](https://github.com/PolyMeilex/rfd). Its window is drawn by OxiGuitar
with [Slint](https://slint.dev). The manual's Credits page lists them all.

## Licence

GPL-3.0-or-later. See LICENSE.
Contributions are welcome under the same terms; see
[CONTRIBUTING.md](CONTRIBUTING.md).

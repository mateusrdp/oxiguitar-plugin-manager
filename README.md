# oxiguitar-plugin-manager

A plugin for [OxiGuitar](https://github.com/mateusrdp/oxiguitar) that manages
the others: every copy of every plugin you have, with its version and the
newest release, and buttons to update a plugin, install a plugin file you
downloaded, or delete a copy.

In plain words: OxiGuitar's own Tools → Plugins lists your plugins and
switches them on and off, and never touches a file. This plugin does the
rest, in your own plugin folder. It is not one of the plugins OxiGuitar
suggests, because it goes to the internet: **each time you open it, it asks
github.com for every plugin's newest release**. It sends nothing about you;
each question names a plugin's repository. Install it only if you want
that.

- **Plugins → Plugin Manager…** lists every plugin file OxiGuitar found when
  it started, each copy on its own line: its version, its folder (yours,
  the one beside OxiGuitar, the system's), and whether OxiGuitar uses it or
  another copy.
- **A `!` after a name** marks a plugin that can reach the network, told
  from the system functions the file uses. This plugin is marked too.
- **The newest release** of each plugin whose homepage is on GitHub, and an
  **Update** button when it is newer than every copy you have. The update
  is checked against the release's SHA256SUMS and goes into your plugin
  folder, replacing your older copies.
- **Install a Plugin File…** copies a `.oxp` file you choose into your
  plugin folder.
- **Delete** removes a copy, except one in a system folder (where packages
  install, and where only the package manager should remove it), which is
  greyed out.

Changes are used the next time you start OxiGuitar, which reads its plugin
folders once.

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
[rfd](https://github.com/PolyMeilex/rfd). The manual's Credits page lists
them all.

## Licence

GPL-3.0-or-later. See LICENSE.
Contributions are welcome under the same terms; see
[CONTRIBUTING.md](CONTRIBUTING.md).

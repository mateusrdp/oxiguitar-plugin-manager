# Installing

You need OxiGuitar first: [its manual](https://mateusrdp.github.io/oxiguitar/) says how to get it. This page is about the Plugin Manager.

## From a release

Each release on the [releases page](https://github.com/mateusrdp/oxiguitar-plugin-manager/releases) has a zip for each system (Linux, Windows and macOS; Linux and Windows for x86_64 and ARM64) and a `SHA256SUMS` file to check the downloads against. Take the zip that matches your system.

## Copying the file yourself

A plugin is a single file ending in `.oxp`. Unzip the release for your system and put `plugin-manager.oxp` in your plugins folder:

| System | Your plugins folder |
| --- | --- |
| Linux | `~/.local/share/oxiguitar/plugins/` |
| Linux, OxiGuitar's Flatpak | `~/.var/app/io.github.mateusrdp.OxiGuitar/data/oxiguitar/plugins/` |
| macOS | `~/Library/Application Support/OxiGuitar/plugins/` |
| Windows | `%APPDATA%\OxiGuitar\plugins\` |

Start OxiGuitar again. **Tools → Plugins…** lists what it found, and **Plugins → Plugin Manager…** opens this plugin. From then on it can update itself.

On macOS, a plugin file you downloaded yourself may be refused as coming from an unverified developer. Clear the download mark and start OxiGuitar again:

```sh
xattr -d com.apple.quarantine ~/Library/Application\ Support/OxiGuitar/plugins/*.oxp
```

## Removing it

Delete `plugin-manager.oxp` from your plugins folder (or press the red X on its line in its own window), and start OxiGuitar again.

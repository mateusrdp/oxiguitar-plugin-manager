# How to use it

Open **Plugins → Plugin Manager…**. The dialog lists every plugin file OxiGuitar found when it started, grouped by plugin, one line a copy:

- the plugin's name, with **`!`** after it when the file can reach the network (see [Architecture and strategy](how-it-works.md) for how that is told);
- its version, its folder (**your folder**, **beside OxiGuitar**, **system**, or a folder your settings list), and whether OxiGuitar uses that copy: **in use**, **not used** because another copy is, **switched off** in Tools → Plugins, or why it **can't be used**;
- on the first line of each plugin, the newest release: **checking…** at first, then **up to date**, **newest: 0.3.0**, **no release for this system**, or why it **can't check**.

Names of copies in a system folder are greyed out, and their **Delete** button is off.

OxiGuitar reads its plugin folders once, when it starts, so whatever you do here is used the next time you start it. The dialog says so after each change. OK and Cancel both just close the dialog: what you did is done.

## See what you have

Open the dialog and read it. A plugin with two lines has two copies; the one **in use** is what OxiGuitar plays and opens files with.

## Update a plugin

When a plugin's newest release is newer than every copy you have, its first line has **Update to 0.3.0**. Press it. The plugin downloads that release's file for your system, checks it against the release's `SHA256SUMS` file, and puts it in your plugin folder under the name its packages use (`gp5.oxp`, say). Your older copies of that plugin in your folder are deleted. A copy in a system folder stays, and OxiGuitar uses the newer one in your folder instead. Start OxiGuitar again to use it.

The Plugin Manager can update itself the same way.

To go back, delete the new copy: the system's copy, if there is one, is used again.

## Install a plugin file you downloaded

Press **Install a Plugin File…**, and choose the `.oxp` file in the file dialog. It must be an OxiGuitar plugin: the dialog says so when it isn't. It is copied into your plugin folder under its own name; a file of the same name there is replaced. Start OxiGuitar again to use it.

A plugin's release has a zip for each system: unzip it first, and choose the `.oxp` inside.

## Delete a copy

Press **Delete** on its line. Copies in your folder can always be deleted. A copy beside OxiGuitar can be deleted when that folder can be changed (a portable copy, not the AppImage, and never inside the macOS app, whose files are signed). A copy in a system folder can't: remove its package with your package manager instead. On Windows a plugin OxiGuitar has loaded is renamed instead, and removed the next time you open the dialog.

## When it can't check

- **"GitHub can't be reached"**: no connection, or a firewall. In OxiGuitar's Flatpak the sandbox has no network at all; to let this plugin check, allow it for OxiGuitar: `flatpak override --user --share=network io.github.mateusrdp.OxiGuitar` (and `flatpak override --user --unshare=network io.github.mateusrdp.OxiGuitar` to take it back).
- **"GitHub's limit of 60 checks an hour is reached"**: GitHub answers that many questions an hour from one internet address without signing in. Opening the dialog again in the same session costs nothing for plugins whose releases haven't changed.
- **"not on GitHub: can't check"**: the plugin's homepage isn't a GitHub repository.
- **"no release for this system"**: none of its releases has a file for your system.

# How to use it

Open **Plugins → Plugin Manager…**. Its window has **Add…** at the top, a list of every plugin file OxiGuitar found when it started, and **Close** at the bottom.

Each line of the list is one copy of a plugin:

- its name, with **`!`** before it when the file can reach the network (see [Architecture and strategy](how-it-works.md) for how that is told), and its version;
- the file's path, under it, in smaller text;
- **Update to …** beside the first copy of a plugin, when there is a newer release;
- the **red X**, which deletes that copy. It is greyed out for a copy in a system folder. Hold the pointer over it to see what it does.

The list's colours and font are OxiGuitar's, light or dark as OxiGuitar is.

OxiGuitar reads its plugin folders once, when it starts, so whatever you do here is used the next time you start it. A line under the list says what the last thing you did came to.

## See what you have

Open the window and read the list. A plugin with two lines has two copies. OxiGuitar uses the one with the highest version, and the one in your folder when the versions are the same; **Tools → Plugins** says which and switches plugins on and off.

## Update a plugin

When a plugin's newest release is newer than every copy you have, its first line has **Update to 0.3.0**. Press it. The plugin downloads that release's file for your system, checks it against the release's `SHA256SUMS` file, and puts it in your plugin folder under the name its packages use (`gp5.oxp`, say). Your older copies of that plugin in your folder are deleted. A copy in a system folder stays, and OxiGuitar uses the newer one in your folder instead. Start OxiGuitar again to use it.

The Plugin Manager can update itself the same way.

To go back, delete the new copy: the system's copy, if there is one, is used again.

## Add a plugin file you downloaded

Press **Add…** and choose the `.oxp` file in the file dialog. It is copied into your plugin folder under its own name once you choose it; a file of the same name there is replaced. It must be an OxiGuitar plugin: the line under the list says so when it isn't. Start OxiGuitar again to use it.

A plugin's release has a zip for each system: unzip it first, and choose the `.oxp` inside.

## Delete a copy

Press the **red X** on its line, then **Delete** in the question that opens (**Cancel** keeps it). Copies in your folder can always be deleted. A copy beside OxiGuitar can be deleted when that folder can be changed (a portable copy, not the AppImage, and never inside the macOS app, whose files are signed). A copy in a system folder can't: remove its package with your package manager instead. On Windows a plugin OxiGuitar has loaded is renamed instead, and removed the next time you open the window.

## When it can't check

The line under the list says why:

- **"GitHub can't be reached"**: no connection, or a firewall. In OxiGuitar's Flatpak the sandbox has no network at all; to let this plugin check, allow it for OxiGuitar: `flatpak override --user --share=network io.github.mateusrdp.OxiGuitar` (and `flatpak override --user --unshare=network io.github.mateusrdp.OxiGuitar` to take it back).
- **"GitHub's limit of 60 checks an hour is reached"**: GitHub answers that many questions an hour from one internet address without signing in. Opening the window again in the same session costs nothing for plugins whose releases haven't changed.

A plugin whose homepage isn't a GitHub repository, or whose releases have no file for your system, has no Update button.

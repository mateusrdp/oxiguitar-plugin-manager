# Architecture and strategy

The plugin is a menu entry (Plugins → Plugin Manager…) and two views of its own UI: the list's window and the question before a delete. Both are components of one Slint file, `ui/manager.slint`, which goes into the plugin file beside its library and which OxiGuitar compiles with Slint's interpreter, from where it unpacks the file, and shows with its own widgets, style, font and colours (plugin interface 1.16; the manifest's `ui` names it). The plugin draws nothing and carries no widget code: it sets the views' properties (the lines, the status line) and hears their callbacks (Add…, Update, the X, Delete, Close), off OxiGuitar's UI thread (`src/ui.rs`). What the list shows and what the buttons do is plain Rust, tested without a window (`src/manager.rs`). OxiGuitar also gives it the list of plugin files it found (`Host::plugin_files`).

## What OxiGuitar tells it

When the window opens, the plugin asks OxiGuitar for every `.oxp` file it examined when it started, copies another file replaces included, each with its folder's kind (yours, beside the program, the system's, or one settings.json lists), the id, name, version and homepage from its manifest, and whether it is used. OxiGuitar also says where your plugin folder is. The plugin writes only there, and deletes only copies that aren't in a system folder (`src/files.rs`).

## Telling whether a plugin can reach the network

No plugin says whether it uses the network: a manifest key for it would be filled in by few authors, and could say anything (Mateus, 24 Nov 2026). The plugin looks at the file instead (`src/network.rs`). A plugin file is an archive holding a shared library (mateusrdp/oxiguitar#347), and the network is reached through the system's libraries, so a plugin that connects anywhere imports their functions: `socket`, `connect` or `getaddrinfo` from the C library on Linux and macOS, the Winsock functions from `ws2_32.dll` or the WinHTTP and WinINet libraries on Windows, or the Network and CFNetwork frameworks on macOS. Rust leaves out of a library the code it never calls, so a plugin that never connects imports none of them. Reading the library's import table takes milliseconds, with the same library OxiGuitar reads manifests with (object), straight from the plugin file's bytes, without unpacking it (`src/oxp.rs`).

It is a strong hint, not a proof. A plugin could find those functions while it runs, or make the system calls itself. The list's `!` means "it imports the network functions".

## Asking for the newest releases

For each plugin whose homepage is `https://github.com/OWNER/REPO`, the plugin asks GitHub's API for that repository's releases (`src/releases.rs`), all at once, off OxiGuitar's threads, each time the window opens, and sets the list again as answers come. Drafts are skipped; pre-releases count, since OxiGuitar's plugins are pre-releases until their 1.0. A release counts when it has a zip for this system, named as OxiGuitar's plugin releases are (`NAME-VERSION-linux-x86_64.zip`, `-linux-aarch64`, `-windows-x86_64`, `-windows-arm64`, `-macos-universal`); its version comes from its tag.

GitHub answers 60 questions an hour from one address without signing in. The plugin keeps each answer's ETag for the session and sends it with the next question: when nothing changed GitHub answers "not modified", which doesn't count.

## Updating

**Update** downloads the release's zip and its `SHA256SUMS`, checks the zip, takes from it the plugin files whose manifest has the plugin's id, and writes each under the name its packages give it (`NAME.oxp`, and `NAME-x86-64-v3.oxp` for a second build such as NAM's) (`src/install.rs`), into the folder of the copy OxiGuitar uses when the plugin may write there (yours, or beside a portable copy of OxiGuitar), and into your plugin folder otherwise. OxiGuitar uses the copy in the folder it searches last, whatever the versions: beside the program, then yours, then the system's (mateusrdp/oxiguitar#347). So a copy inside an AppImage or the macOS app stays in use, and the window says so. The older copies of that plugin in the folder written to are deleted. A file is written under a temporary name first and then renamed into place, so OxiGuitar never finds half a plugin.

A plugin OxiGuitar has loaded stays loaded until it quits. On Linux and macOS replacing or deleting its file is safe: the running program keeps the old contents. Windows doesn't let a loaded file be replaced or deleted, but it can be renamed: the old copy becomes `NAME.oxp.old`, which OxiGuitar ignores, and the plugin removes it the next time its window opens.

## What it doesn't do

- **Nothing runs on its own.** It goes to GitHub only when you open the window, and downloads only when you press Update.
- **It changes nothing outside your plugin folder**, except deleting a copy you choose beside a portable OxiGuitar or in a folder your settings list.
- **No packages.** It never runs a package manager, and never touches what packages installed.
- **Not GitHub? No check.** A plugin hosted elsewhere is listed, and its newest release is unknown.
- **No restart.** OxiGuitar reads its plugin folders when it starts and never unloads a plugin, so changes wait for the next start.

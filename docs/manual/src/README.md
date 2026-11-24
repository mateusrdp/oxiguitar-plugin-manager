# Plugin Manager

The Plugin Manager is a plugin for [OxiGuitar](https://mateusrdp.github.io/oxiguitar/) that looks after your other plugins. It shows every copy of every plugin you have, with its version and the newest release, and lets you update a plugin, install a plugin file you downloaded, or delete a copy.

## Some background

Almost everything OxiGuitar does comes from a plugin: each file format, the sound, the audio outputs. A plugin is one file ending in `.oxp`. OxiGuitar looks for them in a few folders when it starts:

- **your plugin folder**, in your home folder, where you put plugins yourself;
- **the system's folders**, where packages (`.deb`, `.rpm`, a Windows or macOS installer) put them;
- **the folder beside OxiGuitar**, in a portable copy, the AppImage or the macOS app.

The same plugin can be in more than one of them. OxiGuitar uses the copy with the highest version, and your folder's copy when the versions are the same. Its own **Tools → Plugins** lists the plugins and switches them on and off, and never changes a file. This plugin does the rest, in your plugin folder.

## It goes to the internet

OxiGuitar and its other plugins never use the network. This one does: **each time you open it, it asks github.com for the newest release of every plugin whose homepage is on GitHub**, and it downloads a release when you press Update. It sends nothing about you or your files: each question names a plugin's repository, as your browser would when you open its releases page. That is why OxiGuitar doesn't suggest it, no package recommends it, and its manual doesn't list it. Install it only if you want this.



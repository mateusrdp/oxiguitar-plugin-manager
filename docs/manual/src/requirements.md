# Requirements and assumptions

## Requirements

- OxiGuitar with plugin interface 1.16 (the release after 0.22.0), which reads plugin files as archives and shows plugins' own UI in Slint from them. See [the OxiGuitar manual](https://mateusrdp.github.io/oxiguitar/).
- Windows, macOS or Linux.
- A connection to github.com, for the newest releases and updates. Listing, installing and deleting work without one. OxiGuitar's Flatpak has no network unless you allow it ([How to use it](use-cases.md#when-it-cant-check)).

## Assumptions

- A plugin's homepage in its manifest is its GitHub repository, and its releases are made the way OxiGuitar's plugins are: a zip for each system, named after the system, with a `SHA256SUMS` file. A release without `SHA256SUMS` is installed unchecked.
- A release's tag is its version, with or without a `v`.
- Your plugin folder is yours to change. System folders belong to the package manager.

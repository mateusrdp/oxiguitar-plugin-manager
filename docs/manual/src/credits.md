# Credits

Thank you to the people whose work this plugin is built on.

## Code we use

| Project | Authors | What it does here | Licence |
| --- | --- | --- | --- |
| [OxiGuitar's plugin SDK](https://github.com/mateusrdp/oxiguitar) (`oxiguitar-plugin`, and `oxiguitar-host` and `oxiguitar-plugin-test` for the tests) | Mateus Rocha de Paula and the OxiGuitar contributors | The plugin interface, and OxiGuitar's own host to test against | MIT OR Apache-2.0 |
| [ureq](https://github.com/algesten/ureq) | Martin Algesten and contributors | Asking GitHub and downloading releases | MIT OR Apache-2.0 |
| [rustls](https://github.com/rustls/rustls) | The rustls contributors | HTTPS | Apache-2.0 OR ISC OR MIT |
| [ring](https://github.com/briansmith/ring) | Brian Smith and contributors, with code from BoringSSL | The cryptography under HTTPS | Apache-2.0 AND ISC |
| [webpki-roots](https://github.com/rustls/webpki-roots) | The rustls contributors; the list is Mozilla's | The certificates HTTPS trusts | CDLA-Permissive-2.0 |
| [object](https://github.com/gimli-rs/object) | The gimli developers | Reading a plugin file's manifest and imports | Apache-2.0 OR MIT |
| [zip](https://github.com/zip-rs/zip2) | Mathijs van de Nes, Marli Frost, Ryan Levick, Chris Hennick and contributors | Reading a release's zip | MIT |
| [sha2](https://github.com/RustCrypto/hashes) | The RustCrypto developers | Checking downloads against SHA256SUMS | MIT OR Apache-2.0 |
| [semver](https://github.com/dtolnay/semver) | David Tolnay | Comparing versions | MIT OR Apache-2.0 |
| [rfd](https://github.com/PolyMeilex/rfd) | Bartłomiej Maryńczak and contributors | The system's file dialog | MIT |
| [Slint](https://slint.dev) | SixtyFPS GmbH and contributors | The window: OxiGuitar compiles the plugin's `.slint` file with Slint's interpreter and draws it (the plugin doesn't carry Slint) | GPL-3.0-only (one of Slint's licences) |
| [Rust](https://www.rust-lang.org/) and its standard library | The Rust Project | Everything else | MIT OR Apache-2.0 |

## Tools

| Tool | Use |
| --- | --- |
| [mdBook](https://github.com/rust-lang/mdBook) | Builds this manual |
| [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) | Checks the licences of everything we build with |

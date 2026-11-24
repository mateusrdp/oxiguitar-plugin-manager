// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mateus Rocha de Paula and the OxiGuitar contributors

//! The newest release of each plugin, from GitHub (Mateus, 24 Nov 2026: the
//! releases of the repository a plugin's manifest names as its homepage,
//! asked each time the window opens).
//!
//! A plugin whose homepage is `https://github.com/OWNER/REPO` is looked up
//! at `api.github.com/repos/OWNER/REPO/releases`. Pre-releases count, since
//! every OxiGuitar plugin is one until its 1.0; drafts don't. A release
//! counts when it has a zip for this system, named as OxiGuitar's plugin
//! releases name them (`NAME-VERSION-SYSTEM-ARCH.zip`, oxiguitar-packages'
//! `plugin-release.yml`): `linux-x86_64`, `linux-aarch64`,
//! `windows-x86_64`, `windows-arm64` or `macos-universal`.
//!
//! GitHub answers 60 unsigned requests an hour from one address. Each
//! repository is asked once a check, and its answer's ETag kept for the
//! session: asking again with it costs nothing when the releases are the
//! same (GitHub answers 304 Not Modified, which doesn't count).

use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use oxiguitar_plugin::serde_json::{self, Value};

/// The most a release list or a download may be, in bytes: far more than
/// any plugin, far less than memory.
pub const DOWNLOAD_MAX: u64 = 256 * 1024 * 1024;

/// What GitHub is told the requests come from.
const USER_AGENT: &str = concat!("oxiguitar-plugin-manager/", env!("CARGO_PKG_VERSION"));

/// A GitHub repository.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Repo {
    /// Its owner.
    pub owner: String,
    /// Its name.
    pub name: String,
}

impl std::fmt::Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// The repository a homepage names, when it is GitHub's:
/// `https://github.com/OWNER/REPO`, with or without `www.`, a trailing
/// `/`, `.git` or a deeper path.
pub fn repo_of(homepage: &str) -> Option<Repo> {
    let rest = homepage
        .trim()
        .strip_prefix("https://")?
        .trim_start_matches("www.")
        .strip_prefix("github.com/")?;
    let mut parts = rest.split(['/', '?', '#']);
    let owner = parts.next()?;
    let name = parts.next()?;
    let name = name.strip_suffix(".git").unwrap_or(name);
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    (ok(owner) && ok(name)).then(|| Repo {
        owner: owner.to_owned(),
        name: name.to_owned(),
    })
}

/// This system's part of a release's file names.
pub fn target() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos-universal"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "windows-arm64"
    } else if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "linux-aarch64"
    } else {
        "linux-x86_64"
    }
}

/// A release with a file for this system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its version, from its tag.
    pub version: semver::Version,
    /// The zip's name.
    pub zip_name: String,
    /// Where the zip is.
    pub zip: String,
    /// Where its SHA256SUMS file is, if it has one.
    pub sums: Option<String>,
}

/// The newest release in GitHub's list of a repository's releases (JSON)
/// that has a zip for `target`; `None` when none has.
pub fn newest(json: &str, target: &str) -> Result<Option<Release>, String> {
    let list: Value =
        serde_json::from_str(json).map_err(|e| format!("GitHub's answer can't be read ({e})"))?;
    let list = list
        .as_array()
        .ok_or_else(|| "GitHub's answer isn't a list of releases".to_owned())?;
    let suffix = format!("-{target}.zip");
    let mut best: Option<Release> = None;
    for release in list {
        if release["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let Some(tag) = release["tag_name"].as_str() else {
            continue;
        };
        let Ok(version) = semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)) else {
            continue;
        };
        let assets = release["assets"].as_array().map_or(&[][..], Vec::as_slice);
        let url = |a: &Value| a["browser_download_url"].as_str().map(str::to_owned);
        let Some((zip_name, zip)) = assets.iter().find_map(|a| {
            let name = a["name"].as_str().filter(|n| n.ends_with(&suffix))?;
            Some((name.to_owned(), url(a)?))
        }) else {
            continue;
        };
        let sums = assets
            .iter()
            .find(|a| a["name"].as_str() == Some("SHA256SUMS"))
            .and_then(url);
        if best.as_ref().is_none_or(|b| version > b.version) {
            best = Some(Release {
                version,
                zip_name,
                zip,
                sums,
            });
        }
    }
    Ok(best)
}

/// What a check of one repository found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    /// Its newest release with a file for this system.
    Release(Release),
    /// It has no release with a file for this system.
    Nothing,
    /// It couldn't be checked; the text says why.
    Failed(String),
}

/// Asks GitHub, keeping each repository's last answer and its ETag for
/// the session.
#[derive(Debug)]
pub struct Checker {
    agent: ureq::Agent,
    /// By repository: the ETag and the list it came with.
    cache: Mutex<HashMap<Repo, (String, String)>>,
}

impl Default for Checker {
    fn default() -> Checker {
        Checker::new()
    }
}

impl Checker {
    /// A checker with nothing asked yet.
    pub fn new() -> Checker {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .user_agent(USER_AGENT)
            .build()
            .into();
        Checker {
            agent,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, HashMap<Repo, (String, String)>> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The newest release of `repo` with a file for this system.
    pub fn check(&self, repo: &Repo) -> Found {
        match self.releases(repo) {
            Ok(json) => match newest(&json, target()) {
                Ok(Some(r)) => Found::Release(r),
                Ok(None) => Found::Nothing,
                Err(why) => Found::Failed(why),
            },
            Err(why) => Found::Failed(why),
        }
    }

    /// GitHub's list of `repo`'s releases, as JSON text.
    fn releases(&self, repo: &Repo) -> Result<String, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=30",
            repo.owner, repo.name
        );
        let cached = self.cache().get(repo).cloned();
        let mut request = self
            .agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some((etag, _)) = &cached {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request
            .call()
            .map_err(|e| format!("GitHub can't be reached ({e})"))?;
        let status = response.status().as_u16();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        match status {
            304 => cached
                .map(|(_, json)| json)
                .ok_or_else(|| "GitHub said nothing changed, and nothing was kept".to_owned()),
            200 => {
                let etag = header("etag");
                let json = response
                    .body_mut()
                    .with_config()
                    .limit(DOWNLOAD_MAX)
                    .read_to_string()
                    .map_err(|e| format!("GitHub's answer was cut off ({e})"))?;
                if let Some(etag) = etag {
                    self.cache().insert(repo.clone(), (etag, json.clone()));
                }
                Ok(json)
            }
            404 => Err(format!("github.com/{repo} isn't there")),
            403 | 429 if header("x-ratelimit-remaining").as_deref() == Some("0") => Err(limited(
                header("x-ratelimit-reset").and_then(|r| r.parse().ok()),
            )),
            _ => Err(format!("GitHub answered {status}")),
        }
    }

    /// Downloads `url`, at most [`DOWNLOAD_MAX`] bytes.
    pub fn download(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| format!("it can't be downloaded ({e})"))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(format!("it can't be downloaded (GitHub answered {status})"));
        }
        let mut out = Vec::new();
        response
            .body_mut()
            .with_config()
            .limit(DOWNLOAD_MAX)
            .reader()
            .read_to_end(&mut out)
            .map_err(|e| format!("the download was cut off ({e})"))?;
        Ok(out)
    }
}

/// What to say when GitHub's hourly limit is reached; `reset` is when it
/// lifts, in seconds since 1970.
fn limited(reset: Option<u64>) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match reset {
        Some(at) if at > now => format!(
            "GitHub's limit of 60 checks an hour is reached; it lifts in {} min",
            (at - now).div_ceil(60)
        ),
        _ => "GitHub's limit of 60 checks an hour is reached".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_homepage_names_its_repository() {
        let repo = |owner: &str, name: &str| {
            Some(Repo {
                owner: owner.into(),
                name: name.into(),
            })
        };
        assert_eq!(
            repo_of("https://github.com/mateusrdp/oxiguitar-midi"),
            repo("mateusrdp", "oxiguitar-midi")
        );
        assert_eq!(repo_of(" https://www.github.com/a/b.git/ "), repo("a", "b"));
        assert_eq!(
            repo_of("https://github.com/a/b/tree/develop#readme"),
            repo("a", "b")
        );
        for other in [
            "",
            "http://github.com/a/b",
            "https://gitlab.com/a/b",
            "https://github.com/a",
            "https://github.com//b",
            "https://github.com/a b/c",
            "https://github.com.evil.example/a/b",
        ] {
            assert_eq!(repo_of(other), None, "{other}");
        }
    }

    fn release(tag: &str, draft: bool, assets: &[&str]) -> Value {
        serde_json::json!({
            "tag_name": tag,
            "draft": draft,
            "prerelease": true,
            "assets": assets.iter().map(|a| serde_json::json!({
                "name": a,
                "browser_download_url": format!("https://github.com/x/y/releases/download/{tag}/{a}"),
            })).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn the_newest_release_with_a_file_for_this_system_is_taken() {
        let list = Value::Array(vec![
            release("v0.3.0", true, &["oxiguitar-y-0.3.0-linux-x86_64.zip"]),
            release("v0.2.1", false, &["oxiguitar-y-0.2.1-windows-x86_64.zip"]),
            release(
                "v0.2.0",
                false,
                &["oxiguitar-y-0.2.0-linux-x86_64.zip", "SHA256SUMS"],
            ),
            release(
                "v0.10.0-beta.1",
                false,
                &["oxiguitar-y-0.10.0-beta.1-linux-aarch64.zip"],
            ),
            release("not-a-version", false, &["z-linux-x86_64.zip"]),
            release("v0.1.0", false, &["oxiguitar-y-0.1.0-linux-x86_64.zip"]),
        ])
        .to_string();
        let r = newest(&list, "linux-x86_64").unwrap().unwrap();
        assert_eq!(r.version.to_string(), "0.2.0", "drafts don't count");
        assert_eq!(r.zip_name, "oxiguitar-y-0.2.0-linux-x86_64.zip");
        assert!(r.sums.unwrap().ends_with("/v0.2.0/SHA256SUMS"));
        let arm = newest(&list, "linux-aarch64").unwrap().unwrap();
        assert_eq!(arm.version.to_string(), "0.10.0-beta.1");
        assert_eq!(arm.sums, None);
        assert_eq!(newest(&list, "macos-universal").unwrap(), None);
        assert!(newest("{\"message\": \"Not Found\"}", "linux-x86_64").is_err());
        assert!(newest("<html>", "linux-x86_64").is_err());
    }

    #[test]
    fn the_target_is_one_of_the_releases_names() {
        assert!(
            [
                "linux-x86_64",
                "linux-aarch64",
                "windows-x86_64",
                "windows-arm64",
                "macos-universal"
            ]
            .contains(&target())
        );
    }

    #[test]
    fn the_hourly_limit_says_when_it_lifts() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(limited(Some(now + 600)).ends_with("lifts in 10 min"));
        assert!(limited(None).ends_with("is reached"));
        assert!(limited(Some(1)).ends_with("is reached"));
    }
}

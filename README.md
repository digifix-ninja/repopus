<p align="center">
  <img src="ui/logo.svg#gh-light-mode-only" width="140" alt="Repopus, the winking octopus">
  <img src="docs/logo-white.svg#gh-dark-mode-only" width="140" alt="Repopus, the winking octopus">
</p>

<h1 align="center">Repopus</h1>

<p align="center">
  <strong>Keep the GitHub projects you follow on your disk, always at their latest version.</strong><br>
  Follow a creator, an app, a library or a single repository. Repopus checks GitHub for you,
  downloads only what changed, and never deletes anything that disappears.
</p>

<p align="center">
  <a href="../../releases/latest">Download</a> ·
  <a href="https://repopus.app">Website</a> ·
  <a href="docs/DEVELOPMENT.md">Build from source</a>
</p>

<p align="center">
  <img alt="macOS" src="https://img.shields.io/badge/macOS-11%2B-111?logo=apple">
  <img alt="Windows" src="https://img.shields.io/badge/Windows-10%2F11-111?logo=windows">
  <img alt="Linux" src="https://img.shields.io/badge/Linux-deb%20·%20rpm%20·%20AppImage-111?logo=linux&logoColor=white">
  <img alt="Licence: MIT" src="https://img.shields.io/badge/licence-MIT-111">
</p>

<img src="docs/screenshot-light.png#gh-light-mode-only" alt="Repopus showing repositories grouped by creator, with up-to-date, update and missing marks">
<img src="docs/screenshot-dark.png#gh-dark-mode-only" alt="Repopus showing repositories grouped by creator, with up-to-date, update and missing marks">

---

## What it does

Apps you run from source, libraries you build on, themes, fonts, datasets: whatever someone
keeps on GitHub, Repopus keeps their latest version on your computer. Paste a link and it takes
care of the rest:

- **A creator**, like `github.com/username`, follows everything that person or organisation
  publishes, including new repositories as they appear.
- **A single repository**, like `github.com/username/project`, adds just that one. Add
  `/tree/<branch>` to follow a specific branch.

Every creator gets their own category, with one line per repository:

| Mark | Meaning |
|:---:|---|
| ✅ | Up to date |
| 🟧 | A newer version is available, or it hasn't been downloaded yet |
| 🟥 | Gone from GitHub (your copy is **kept**), missing on your computer, or an error |

## Features

- **Checks before it downloads.** Only repositories whose latest commit differs from your copy
  are downloaded. Checks that find nothing new cost (almost) nothing.
- **Automatic updates.** Checks every hour by default, quietly from the menu bar or system tray.
  Choose per repository whether it updates by itself.
- **Zip or folder.** Keep each repository as a zip archive or as an unpacked folder.
- **Replace or keep versions.** One up-to-date copy, or every version side by side, named by
  date and commit, optionally limited to the last *N*.
- **Your folders.** Everything goes into `~/Repopus/<account>/` by default, or anywhere you choose,
  per repository.
- **Safe by design.** Downloads are verified before they replace anything, so an interrupted
  download never breaks a copy. Repositories that vanish from GitHub stay on your computer.
  Repopus only ever removes files it created itself.
- **Foldable categories**, filters, search, live download progress and **Update all**, globally
  or per account.
- **Light, dark grey, or follow the system.**
- **macOS, Windows and Linux.**

## Install

Download the latest version for your system from the [**Releases**](../../releases/latest) page:

| System | File |
|---|---|
| macOS (Apple Silicon) | `Repopus-macOS-Apple-Silicon.dmg` |
| macOS (Intel) | `Repopus-macOS-Intel.dmg` |
| Windows | `Repopus-Windows-Setup.exe` or `Repopus-Windows.msi` |
| Linux | `Repopus-Linux.AppImage` (any distribution), `.deb` (Debian, Ubuntu) or `.rpm` (Fedora, openSUSE) |

> **First start:** the macOS version is signed and notarised by Apple. The Windows installers
> aren't signed yet: choose **More info → Run anyway** if SmartScreen asks.
> On Linux, the tray icon needs AppIndicator support (on GNOME: the "AppIndicator" extension).

## GitHub token (optional)

Without a token, GitHub allows 60 requests an hour and only public repositories. That's enough
for a handful of repositories. For more, or for your private ones, add a token in **Settings**:

- Repopus uses the login of the [GitHub CLI](https://cli.github.com) automatically if you have it
  (`gh auth login`), or
- create a [fine-grained token](https://github.com/settings/personal-access-tokens/new) with
  read-only access to **Contents** and paste it into Settings.

The token is stored in your system's own keychain (macOS Keychain, Windows Credential Manager or
the Linux Secret Service), never in a plain file.

## Privacy

Repopus only talks to GitHub. No accounts, no tracking, no analytics. Your settings stay in a
small file on your computer.

## Build from source

Repopus is written in Rust with [Tauri](https://tauri.app); the interface is plain HTML, CSS
and JavaScript, with no Node.js needed.

```sh
cargo install tauri-cli --version "^2" --locked
cargo tauri dev       # run it
cargo tauri build     # make installers for your system
```

See [**docs/DEVELOPMENT.md**](docs/DEVELOPMENT.md) for requirements per system, the project
layout and how releases are built.

## Contributing

Bug reports, ideas and pull requests are welcome. Please open an issue first for bigger changes.

## Licence

[MIT](LICENSE) © 2026 [stean.ART](https://stean.art)

Repopus isn't affiliated with GitHub, Inc. GitHub is a trademark of GitHub, Inc.;
Git is a trademark of the Software Freedom Conservancy.

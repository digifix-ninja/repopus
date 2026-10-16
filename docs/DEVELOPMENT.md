# Developing Repopus

Repopus is a [Tauri 2](https://tauri.app) desktop app: a Rust sync engine (`core/`), a thin Rust
app around it (`src-tauri/`) and a plain HTML/CSS/JavaScript interface (`ui/`). There's no
Node.js and no frontend build step.

## Requirements

- **Rust 1.85+** (`rustup update`)
- **Tauri CLI:** `cargo install tauri-cli --version "^2" --locked`
- Per system:
  - **macOS:** Xcode command line tools (`xcode-select --install`)
  - **Windows:** Microsoft C++ Build Tools ("Desktop development with C++"); WebView2 is built
    into Windows 10/11
  - **Linux (Debian/Ubuntu):**
    `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libdbus-1-dev pkg-config build-essential`

## Running and building

```sh
cargo tauri dev       # run in development
cargo tauri build     # installers for the system you're on
```

Each system builds its own installers; a Mac can't build the Windows or Linux versions locally.

| | macOS | Windows | Linux |
|---|---|---|---|
| Installers | `.app`, `.dmg` | `.msi`, `-setup.exe` | `.deb`, `.rpm`, `.AppImage` |
| Token stored in | Keychain | Credential Manager | Secret Service (GNOME Keyring, KWallet) |
| Tray icon | menu bar (black, tinted by macOS) | system tray (white) | system tray (white) |
| "Show in…" | Finder (item selected) | Explorer (item selected) | file manager (folder opened) |

If the project sits on a disk without file-lock support (some external or network drives),
incremental compilation is already switched off in `Cargo.toml`. For faster builds, put the
build output on the internal disk, e.g. `export CARGO_TARGET_DIR="$HOME/Library/Caches/repopus-target"`.

## Releases (all three systems)

`.github/workflows/build.yml` is a GitHub Actions workflow that builds macOS (Apple Silicon and
Intel), Windows and Linux on GitHub's servers.

- **Push a version tag** (`git tag v0.1.0 && git push origin v0.1.0`): the installers are attached
  to a **draft** release. Nothing is public until you press *Publish*.
- **Run it by hand** under *Actions → Build Repopus → Run workflow*: the installers are kept as
  downloadable artifacts of the run.

The installers aren't code-signed yet. Signing needs an Apple Developer account and a Windows
code-signing certificate, which can be added to the workflow later.

## How it works

The rules the engine follows:

- **Check before downloading.** It compares the latest commit on GitHub with the copy on disk.
  Accounts are listed in pages of 100, and repositories whose `pushed_at` didn't move aren't
  asked again. Everything else uses ETags, so an unchanged repository costs nothing against
  GitHub's rate limit.
- **Safe swaps.** Downloads go to a temporary folder next to the destination, are verified
  (and unpacked), and only then replace the old copy.
- **Never deletes what vanished upstream.** Repositories or accounts that disappear from GitHub
  are marked as gone; their files stay.
- **Only removes its own files:** the previous copy in *Replace* mode (also when switching
  between zip and folder), and versions beyond *Keep last N* if a limit is set. Anything
  unknown in the way is renamed `… (kept <date>)`.
- **Windows-safe names.** File names Windows forbids (`aux.c`, `a:b.txt`, trailing dots…) get a
  safe name when unpacking on Windows, so one odd file can't stop a whole repository.

Where copies go, inside the repository's folder (default `~/Repopus/<owner>/`):

| Keep as | Replace | Keep versions |
|---|---|---|
| Zip | `<name>.zip` | `<name> versions/<name>_<date>_<sha7>.zip` |
| Folder | `<name>/` | `<name> versions/<date>_<sha7>/` |

## Project layout

```
core/src/                 sync engine (no UI)
  model.rs                  stored data and what the window receives
  link.rs                   Link::parse – understanding pasted links
  github.rs                 GitHub – REST client (ETags, rate limits, zip streaming)
  repo.rs                   impl Repo – branch, status, row for the window
  placement.rs              Placement – where a copy goes on disk
  archive.rs                ZipFile – validate and unpack (Windows-safe names)
  installer.rs              Installer – put a copy in place safely
  files.rs                  small disk helpers
  store.rs                  StateFile – load / atomic save
  engine/
    mod.rs                  Engine – shared state, reading, settings
    library.rs              follow accounts, add/remove repositories
    checker.rs              ask GitHub what's new
    updater.rs              download, verify, install, keep versions
src-tauri/src/            the desktop app (macOS, Windows, Linux)
  main.rs                   start-up and wiring
  state.rs                  AppState – what commands can reach
  commands.rs               everything the window can ask for
  events.rs                 engine events → window events
  tray.rs                   Tray – menu-bar / system-tray icon
  scheduler.rs              Scheduler – automatic checks
  token.rs                  Token – system keychain / GitHub CLI
  desktop.rs                Desktop – show in Finder/Explorer, open links (per system)
  look.rs                   Look – light / dark window frame
  about.rs                  About – version, websites, developer, licence, allowed links
src-tauri/tauri.conf.json   app configuration
ui/                       the interface (plain HTML/CSS/JS modules)
  index.html, styles.css, logo.svg
  js/main.js                App – creates views, connects to Rust
  js/api.js                 Api – every command and event
  js/store.js               AppStore – data, filters, categories, folding
  js/platform.js            Platform – macOS / Windows / Linux names (Finder, Explorer…)
  js/theme.js               Theme – system / light / dark grey
  js/format.js, icons.js, toast.js
  js/views/                 Header, RepoList, Progress, Sheet, RepoPanel, SettingsPanel, AboutPanel
.github/workflows/build.yml  GitHub Actions workflow for all three systems
docs/                     this guide and the README screenshots
```

Code style: small objects with one job each, and a short comment on every function.

## Where settings live

`repopus.json` in the app's data folder:

- macOS: `~/Library/Application Support/ninja.digifix.repopus/`
- Windows: `%APPDATA%\ninja.digifix.repopus\`
- Linux: `~/.local/share/ninja.digifix.repopus/`

## Changing the About information

Websites, developer and the link to the source code are constants in
`src-tauri/src/about.rs`. The licence text is in `LICENSE`.

// The operating system the window runs on, and the names it uses for things.

const agent = navigator.userAgent;
const os = /Macintosh|Mac OS X/.test(agent) ? "mac" : /Windows/.test(agent) ? "windows" : "linux";

export class Platform {
  /** "mac", "windows" or "linux". */
  static get os() {
    return os;
  }

  /** "Finder", "Explorer" or "file manager". */
  static get fileManager() {
    return { mac: "Finder", windows: "Explorer", linux: "file manager" }[os];
  }

  /** Where a saved GitHub token is kept. */
  static get keychain() {
    return { mac: "your Mac's Keychain", windows: "Windows Credential Manager", linux: "your system keyring" }[os];
  }

  /** "menu bar" on macOS, "system tray" elsewhere. */
  static get trayName() {
    return os === "mac" ? "menu bar" : "system tray";
  }

  /** Marks <body> with the platform, so CSS can adapt (e.g. room for macOS window buttons). */
  static markBody() {
    document.body.classList.add(`platform-${os}`);
  }
}

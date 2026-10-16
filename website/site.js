// Repopus website: the download buttons, the live window, and the copy button.

/**
 * The installers of the current release. On a new version, change `version` (and the sizes);
 * the files are the names Tauri gives them, attached to the GitHub release `v<version>`.
 */
const RELEASE = {
  version: "0.1.0",
  base: "https://github.com/digifix-ninja/repopus/releases/download",
  files: [
    { os: "mac", label: "Apple Silicon", note: "M1 and later", file: "Repopus_{v}_aarch64.dmg", size: "3.3 MB", main: true },
    { os: "mac", label: "Intel", note: "older Macs", file: "Repopus_{v}_x64.dmg" },
    { os: "windows", label: "Installer", note: ".exe", file: "Repopus_{v}_x64-setup.exe", main: true },
    { os: "windows", label: "MSI package", note: ".msi", file: "Repopus_{v}_x64_en-US.msi" },
    { os: "linux", label: "AppImage", note: "any distribution", file: "Repopus_{v}_amd64.AppImage", main: true },
    { os: "linux", label: "Debian, Ubuntu", note: ".deb", file: "Repopus_{v}_amd64.deb" },
    { os: "linux", label: "Fedora, openSUSE", note: ".rpm", file: "Repopus-{v}-1.x86_64.rpm" },
  ],

  /** The download link of one installer. */
  url(item) {
    const name = item.file.replace("{v}", this.version);
    return `${this.base}/v${this.version}/${name}`;
  },

  /** The installer to offer first on a system. */
  main(os) {
    return this.files.find((item) => item.os === os && item.main);
  },
};

/** Puts a download button for every installer into the system cards. */
class Downloads {
  static render() {
    document.getElementById("release-version").textContent = RELEASE.version;
    document.querySelectorAll(".get[data-for]").forEach((box) => {
      box.innerHTML = RELEASE.files
        .filter((item) => item.os === box.dataset.for)
        .map((item) => Downloads.button(item))
        .join("");
    });
  }

  /** One button: label, a short note and the size if known. */
  static button(item) {
    const detail = [item.note, item.size].filter(Boolean).join(", ");
    return `<a class="file${item.main ? " main" : ""}" href="${RELEASE.url(item)}" download>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10"/></svg>
      <span>${item.label}<small>${detail}</small></span></a>`;
  }
}

/** Works out which system the visitor is on: "mac", "windows", "linux" or null. */
class Visitor {
  static get os() {
    const ua = navigator.userAgent;
    if (/iPhone|iPad|Android/i.test(ua)) return null; // phones get the general button
    if (/Mac/i.test(ua)) return "mac";
    if (/Win/i.test(ua)) return "windows";
    if (/Linux|X11/i.test(ua)) return "linux";
    return null;
  }

  /** Points the main button at the installer for this system and highlights its card. */
  static personalise() {
    const names = { mac: "macOS", windows: "Windows", linux: "Linux" };
    const os = Visitor.os;
    if (!os) return; // the button keeps pointing to the download section
    const button = document.getElementById("download-main");
    button.href = RELEASE.url(RELEASE.main(os));
    button.setAttribute("download", "");
    document.getElementById("download-label").textContent = `Download for ${names[os]}`;
    document.querySelector(`.systems [data-os="${os}"]`)?.classList.add("here");
  }
}

/** Plays a small update round in the window: queue, download, done. Then again, now and then. */
class Demo {
  constructor(root) {
    this.root = root;
    this.rows = [...root.querySelectorAll("[data-demo]")];
    this.summary = document.getElementById("demo-summary");
    this.stage = root.closest(".window-stage");
    this.calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  }

  /** Starts the loop (or, with reduced motion, simply shows everything up to date). */
  start() {
    if (this.calm) {
      this.rows.forEach((row) => this.set(row, "up"));
      this.summary.textContent = "All up to date";
      return;
    }
    setTimeout(() => this.round(), 1400);
  }

  /** One round: every outdated row waits, downloads, and turns green, one after another. */
  async round() {
    this.summary.textContent = "Updating…";
    this.rows.forEach((row) => this.set(row, "wait"));
    for (const row of this.rows) {
      await this.download(row);
      this.set(row, "up");
      this.ripple(row);
      await Demo.pause(500);
    }
    this.summary.textContent = "All up to date";
    await Demo.pause(5200);
    // Someone pushed new commits: back to orange, and round we go.
    this.rows.forEach((row) => this.set(row, "update"));
    this.summary.textContent = "2 updates";
    await Demo.pause(2600);
    this.round();
  }

  /** Fills a row's download bar over a couple of seconds. */
  download(row) {
    const state = row.querySelector(".state");
    const total = 2.4 + Math.random() * 3; // MB
    this.set(row, "down");
    return new Promise((done) => {
      const began = performance.now();
      const length = 2200 + Math.random() * 900;
      const tick = (now) => {
        const part = Math.min(1, (now - began) / length);
        state.style.setProperty("--p", `${(part * 100).toFixed(1)}%`);
        state.dataset.text = `${(part * total).toFixed(1)} of ${total.toFixed(1)} MB`;
        part < 1 ? requestAnimationFrame(tick) : done();
      };
      requestAnimationFrame(tick);
    });
  }

  /** Changes a row's mark and status text. */
  set(row, state) {
    row.dataset.state = state;
  }

  /** Sends a ring out from a row's mark when it has just finished. */
  ripple(row) {
    const mark = row.querySelector(".mark").getBoundingClientRect();
    const stage = this.stage.getBoundingClientRect();
    const ring = document.createElement("span");
    ring.className = "ripple";
    ring.style.left = `${mark.left - stage.left + mark.width / 2}px`;
    ring.style.top = `${mark.top - stage.top + mark.height / 2}px`;
    this.stage.append(ring);
    ring.addEventListener("animationend", () => ring.remove());
  }

  /** Waits a moment. */
  static pause(ms) {
    return new Promise((done) => setTimeout(done, ms));
  }
}

/** The "Copy" button on the terminal. */
class CopyButtons {
  static bind() {
    document.querySelectorAll("[data-copy]").forEach((button) => {
      button.addEventListener("click", async () => {
        const text = document.getElementById(button.dataset.copy).innerText;
        try {
          await navigator.clipboard.writeText(text);
          button.textContent = "Copied";
        } catch {
          button.textContent = "Select and copy";
        }
        setTimeout(() => (button.textContent = "Copy"), 1800);
      });
    });
  }
}

Downloads.render();
Visitor.personalise();
CopyButtons.bind();
const demo = document.getElementById("demo");
if (demo) new Demo(demo).start();

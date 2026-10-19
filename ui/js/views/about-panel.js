// Side panel about the app: version, links, updates, developer, licence, privacy.

import { Api } from "../api.js";
import { Format } from "../format.js";
import { Icons } from "../icons.js";
import { Sheet } from "./sheet.js";
import { Platform } from "../platform.js";

const esc = Format.esc;

export class AboutPanel extends Sheet {
  constructor(toast) {
    super(document.querySelector("#about"));
    this.toast = toast;
    this.info = null;
    this.update = null;
    this.checking = false;
    this.el.addEventListener("click", e => this.onClick(e));
  }

  /** Opens the panel (the app's details are fetched once). */
  async show() {
    this.open();
    if (!this.info) this.info = await Api.about().catch(() => null);
    this.render();
  }

  /** The latest word on a newer version of Repopus (from the window or a check). */
  setUpdate(status) {
    this.update = status;
    this.render();
  }

  /** Draws the panel. */
  render() {
    if (!this.isOpen) return;
    const a = this.info || { name: "Repopus", version: "", developer: "", website: "", licence: "MIT" };
    const host = url => (url || "").replace(/^https:\/\//, "");
    const link = url => `<a class="link" data-url="${esc(url)}">${esc(host(url))}</a>`;
    const row = (label, value) => value ? `<div class="about-row"><span class="lbl">${label}</span><span>${value}</span></div>` : "";

    this.renderKeepingScroll(`
      <div class="sheet-head">
        <div class="t"><h3>About</h3></div>
        ${Sheet.closeButton(Icons.close)}
      </div>
      <div class="sheet-body about">
        <div class="about-hero">
          <img src="logo.svg" alt="">
          <h2>${esc(a.name)}</h2>
          <div class="muted">Version ${esc(a.version)}</div>
          <p>Keeps the GitHub projects you follow up to date on your computer, and never deletes
             anything that disappears from GitHub.</p>
        </div>

        <div class="section">
          <h4>${esc(a.name)}</h4>
          <div class="card">
            ${row("Website", a.app_website && link(a.app_website))}
            ${row("GitHub", a.source_code && link(a.source_code))}
          </div>
        </div>

        <div class="section">
          <h4>Updates</h4>
          <div class="card"><div class="field about-update">${this.updateText()}</div></div>
        </div>

        <div class="section">
          <h4>Developer</h4>
          <div class="card">
            ${row("Developer", `<strong>${esc(a.developer)}</strong>`)}
            ${row("Website", a.website && link(a.website))}
            ${row("Email", a.email && `<a class="link" data-url="mailto:${esc(a.email)}">${esc(a.email)}</a>`)}
          </div>
        </div>

        <div class="section">
          <h4>Open source</h4>
          <div class="card"><div class="field">
            <div class="help">${esc(a.name)} is free and open-source software under the ${esc(a.licence)} licence.
              You may use, change and share it.</div>
            <div class="help">Built with Rust and Tauri.</div>
          </div></div>
        </div>

        <div class="section">
          <h4>Privacy</h4>
          <div class="card"><div class="field">
            <div class="help">${esc(a.name)} only talks to GitHub. No accounts, no tracking, no analytics.
              A GitHub token, if you add one, stays in ${esc(Platform.keychain)}.</div>
          </div></div>
        </div>

        <p class="fineprint">© ${esc(a.copyright_year || "")} ${esc(a.developer)} ·
          ${esc(a.name)} is not affiliated with GitHub, Inc. GitHub is a trademark of GitHub, Inc.;
          Git is a trademark of the Software Freedom Conservancy.</p>
      </div>`);
  }

  /** The "Updates" card: a newer version, "up to date", or a failed check. */
  updateText() {
    const u = this.update;
    const button = `<button class="btn small" data-check-update ${this.checking ? "disabled" : ""}>
      ${this.checking ? "Checking…" : "Check for updates"}</button>`;
    let text = `<span class="muted">Repopus looks for a new version on GitHub twice a day.</span>`;
    if (u?.available) {
      text = `<span><strong>Version ${esc(u.available.version)} is available.</strong></span>
        <button class="btn small primary" data-url="${esc(u.available.url)}">Download the update</button>`;
    } else if (u?.error) {
      text = `<span class="muted">Couldn't check: ${esc(u.error)}</span>`;
    } else if (u?.checked) {
      text = `<span>You have the latest version.</span>`;
    }
    return `<div class="about-update-line">${text}</div><div>${button}</div>`;
  }

  /** Asks GitHub now; the answer also reaches the banner through the "app-update" event. */
  async checkUpdate() {
    this.checking = true;
    this.render();
    try {
      this.update = await Api.checkAppUpdate();
    } catch (error) {
      this.toast.error(error);
    } finally {
      this.checking = false;
      this.render();
    }
  }

  /** Close button and links. */
  onClick(event) {
    const t = event.target.closest("button, a");
    if (!t) return;
    if (t.hasAttribute("data-close")) return this.close();
    if (t.hasAttribute("data-check-update")) return this.checkUpdate();
    if (t.dataset.url) Api.openUrl(t.dataset.url).catch(e => this.toast.error(e));
  }
}

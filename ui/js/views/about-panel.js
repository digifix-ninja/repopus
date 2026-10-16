// Side panel about the app: version, developer, licence, privacy.

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
    this.el.addEventListener("click", e => this.onClick(e));
  }

  /** Opens the panel (the app's details are fetched once). */
  async show() {
    this.open();
    if (!this.info) this.info = await Api.about().catch(() => null);
    this.render();
  }

  /** Draws the panel. */
  render() {
    if (!this.isOpen) return;
    const a = this.info || { name: "Repopus", version: "", developer: "", website: "", licence: "MIT" };
    const host = url => (url || "").replace(/^https:\/\//, "");
    const source = a.source_code
      ? `<a class="link" data-url="${esc(a.source_code)}">View the source code on GitHub</a>`
      : `<span class="muted">The source code will be published on GitHub.</span>`;

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
          ${a.app_website ? `<div class="about-site"><a class="link" data-url="${esc(a.app_website)}">${esc(host(a.app_website))}</a></div>` : ""}
          <p>Keeps local copies of your GitHub repositories up to date — and never deletes
             anything that disappears from GitHub.</p>
        </div>

        <div class="section">
          <h4>Developer</h4>
          <div class="card"><div class="field">
            <div class="lbl">${esc(a.developer)}</div>
            <div class="help"><a class="link" data-url="${esc(a.website)}">${esc(host(a.website))}</a></div>
          </div></div>
        </div>

        <div class="section">
          <h4>Open source</h4>
          <div class="card"><div class="field">
            <div class="help">${esc(a.name)} is free and open-source software under the ${esc(a.licence)} licence.
              You may use, change and share it.</div>
            <div class="help">${source}</div>
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

  /** Close button and links. */
  onClick(event) {
    const t = event.target.closest("button, a");
    if (!t) return;
    if (t.hasAttribute("data-close")) return this.close();
    if (t.dataset.url) Api.openUrl(t.dataset.url).catch(e => this.toast.error(e));
  }
}

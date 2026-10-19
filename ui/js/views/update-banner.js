// Bar under the title bar when a newer version of Repopus has been published on GitHub.

import { Api } from "../api.js";
import { Format } from "../format.js";

const esc = Format.esc;
const DISMISSED = "repopus.dismissedUpdate"; // the version the user chose "Later" for

export class UpdateBanner {
  constructor(toast) {
    this.el = document.querySelector("#app-update");
    this.toast = toast;
    this.release = null;
    this.el.addEventListener("click", e => this.onClick(e));
  }

  /** Shows the bar for a newer version, unless the user put that version off. */
  show(status) {
    this.release = status?.available || null;
    const dismissed = this.dismissed();
    if (!this.release || dismissed === this.release.version) {
      this.el.hidden = true;
      return;
    }
    this.el.innerHTML = `
      <span class="msg"><strong>Repopus ${esc(this.release.version)} is available.</strong>
        You have version ${esc(status.current)}.</span>
      <button class="btn small primary" data-download>Download the update</button>
      <button class="link-btn" data-later>Later</button>`;
    this.el.hidden = false;
  }

  /** "Download" opens the release page; "Later" hides the bar until the next version. */
  onClick(event) {
    const t = event.target.closest("button");
    if (!t || !this.release) return;
    if (t.hasAttribute("data-download")) Api.openUrl(this.release.url).catch(e => this.toast.error(e));
    if (t.hasAttribute("data-later")) {
      try { localStorage.setItem(DISMISSED, this.release.version); } catch {}
      this.el.hidden = true;
    }
  }

  dismissed() {
    try { return localStorage.getItem(DISMISSED); } catch { return null; }
  }
}

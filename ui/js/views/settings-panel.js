// Side panel with the app settings.

import { Api } from "../api.js";
import { Format } from "../format.js";
import { Icons } from "../icons.js";
import { Sheet } from "./sheet.js";
import { Platform } from "../platform.js";

const esc = Format.esc;

const INTERVALS = [15, 30, 60, 180, 360, 720, 1440];
const PARALLEL = [1, 2, 3, 4, 6, 8];
const TOKEN_PAGE = "https://github.com/settings/personal-access-tokens/new";

export class SettingsPanel extends Sheet {
  constructor(store, toast, actions) {
    super(document.querySelector("#settings"));
    this.store = store;
    this.toast = toast;
    this.actions = actions; // { reload(), openAbout(), chooseRepos(owner) }
    this.tokenStatus = null;
    this.el.addEventListener("click", e => this.onClick(e));
    this.el.addEventListener("change", e => this.onChange(e));
  }

  /** Opens the panel and looks up the token status. */
  async show() {
    this.open();
    this.render();
    this.tokenStatus = await Api.tokenStatus().catch(() => null);
    this.render();
  }

  /** Opens or closes the panel. */
  toggleOpen() {
    if (this.isOpen) this.close();
    else this.show();
  }

  // ---------------------------------------------------------------- drawing

  /** Redraws the panel (only while open). */
  render() {
    if (!this.isOpen || !this.store.settings) return;
    this.renderKeepingScroll(`
      <div class="sheet-head">
        <div class="t"><h3>Settings</h3><div class="owner">Applies to the whole app</div></div>
        ${Sheet.closeButton(Icons.close)}
      </div>
      <div class="sheet-body">
        ${this.tokenHtml()}
        ${this.folderHtml()}
        ${this.checkingHtml()}
        ${this.defaultsHtml()}
        ${this.accountsHtml()}
        ${this.appHtml()}
        <div class="settings-about"><button class="btn small" data-about>About Repopus</button></div>
      </div>`);
  }

  /** GitHub token: status, field, link to create one. */
  tokenHtml() {
    const removable = this.tokenStatus?.source === "keychain";
    return `
      <div class="section">
        <h4>GitHub access</h4>
        <div class="card">
          <div class="field">
            <div class="lbl">Personal access token</div>
            ${this.tokenLine()}
            <div class="ctrl">
              <input class="text mono" type="password" id="token-input" placeholder="ghp_… or github_pat_…" spellcheck="false">
              <button class="btn small primary" data-token-save>Save</button>
              ${removable ? `<button class="btn small" data-token-remove>Remove</button>` : ""}
            </div>
            <div class="help"><a class="link" data-url="${TOKEN_PAGE}">Create a token on GitHub</a>. Read-only access to “Contents” is all it needs.</div>
          </div>
        </div>
      </div>`;
  }

  /** Explains which token is in use. */
  tokenLine() {
    const status = this.tokenStatus;
    if (!status) return `<div class="help">Checking…</div>`;
    if (status.error) return `<div class="help warn">${esc(status.error)}</div>`;
    const who = status.login ? ` (${esc(status.login)})` : "";
    switch (status.source) {
      case "keychain":
        return `<div class="help">Using the token saved in ${Platform.keychain}${who}. 5,000 requests per hour, and private repositories work.</div>`;
      case "github_cli":
        return `<div class="help">Using your GitHub CLI login${who}. Save a token here to use a different one.</div>`;
      default:
        return `<div class="help">No token: GitHub allows 60 requests per hour and public repositories only. Checks that find nothing new cost almost nothing, but the first check of a large account can run out.</div>`;
    }
  }

  /** Main folder. */
  folderHtml() {
    const base = this.store.settings.base_folder;
    return `
      <div class="section">
        <h4>Where repositories go</h4>
        <div class="card">
          <div class="field">
            <div class="lbl">Main folder</div>
            <div class="ctrl"><div class="path selectable" title="${esc(base)}">${Format.path(base)}</div><button class="btn small" data-choose-base>Choose…</button></div>
            <div class="help">Each account gets a subfolder. Any repository can have its own folder instead.</div>
          </div>
        </div>
      </div>`;
  }

  /** Automatic checks, interval, parallel downloads. */
  checkingHtml() {
    const s = this.store.settings;
    const intervals = INTERVALS.map(m => `<option value="${m}" ${s.check_interval_minutes === m ? "selected" : ""}>${SettingsPanel.intervalLabel(m)}</option>`).join("");
    const parallel = PARALLEL.map(n => `<option value="${n}" ${s.concurrent_downloads === n ? "selected" : ""}>${n}</option>`).join("");
    return `
      <div class="section">
        <h4>Checking</h4>
        <div class="card">
          <div class="field inline">
            <div><div class="lbl">Check automatically</div><div class="help">Last check: ${esc(Format.ago(this.store.app.last_check))}</div></div>
            <div class="ctrl">${Sheet.toggle("auto_check", s.auto_check)}</div>
          </div>
          <div class="field inline">
            <div class="lbl">How often</div>
            <div class="ctrl"><select class="text" data-number="check_interval_minutes">${intervals}</select></div>
          </div>
          <div class="field inline">
            <div class="lbl">Downloads at the same time</div>
            <div class="ctrl"><select class="text" data-number="concurrent_downloads">${parallel}</select></div>
          </div>
        </div>
      </div>`;
  }

  /** Defaults for repositories added from now on. */
  defaultsHtml() {
    const s = this.store.settings;
    const d = s.defaults;
    return `
      <div class="section">
        <h4>New repositories</h4>
        <div class="card">
          <div class="field">
            <div class="help">These settings are given to repositories added from now on. Repositories you
              already have keep their own settings (click a repository to change it).</div>
            <div class="ctrl"><button class="btn small" data-apply-defaults>Apply to all existing repositories</button></div>
          </div>
          <div class="field inline">
            <div class="lbl">Update automatically</div>
            <div class="ctrl">${Sheet.toggle("defaults.auto_update", d.auto_update)}</div>
          </div>
          <div class="field">
            <div class="lbl">Keep as</div>
            <div class="ctrl">${Sheet.segmented("defaults.format", d.format, [["zip", "Zip archive"], ["folder", "Unpacked folder"]])}</div>
          </div>
          <div class="field">
            <div class="lbl">When a new version arrives</div>
            <div class="ctrl">${Sheet.segmented("defaults.mode", d.mode, [["replace", "Replace the old copy"], ["keep_versions", "Keep every version"]])}</div>
          </div>
          <div class="field inline">
            <div class="lbl">Include forks</div>
            <div class="ctrl">${Sheet.toggle("include_forks", s.include_forks)}</div>
          </div>
          <div class="field inline">
            <div class="lbl">Include archived repositories</div>
            <div class="ctrl">${Sheet.toggle("include_archived", s.include_archived)}</div>
          </div>
        </div>
      </div>`;
  }

  /** Followed accounts, with "choose repositories" and unfollow. */
  accountsHtml() {
    const counts = {};
    for (const repo of this.store.all()) if (repo.source) counts[repo.source] = (counts[repo.source] || 0) + 1;
    const accounts = this.store.sources.length
      ? this.store.sources.map(source => this.accountHtml(source, counts[source.id] || 0)).join("")
      : `<div class="field"><div class="help">You're not following any accounts yet.</div></div>`;
    return `<div class="section"><h4>Followed accounts</h4><div class="card">${accounts}</div></div>`;
  }

  /** One followed account. */
  accountHtml(source, count) {
    const kind = source.account_type === "Organization" ? "organisation" : "user";
    const notes = [];
    if (source.ignored.length) notes.push(`${Format.repositories(source.ignored.length)} not tracked`);
    notes.push(source.add_new ? "new repositories are added automatically" : "new repositories are not added");
    const help = notes.join(" · ");
    return `
      <div class="account">
        <div class="top">
          <div class="who"><b>${esc(source.owner)}</b> <span class="muted">· ${kind} · ${Format.repositories(count)}</span></div>
          <button class="btn small" data-choose="${esc(source.owner)}">Choose repositories…</button>
          <button class="btn small danger" data-unfollow="${esc(source.id)}">Unfollow</button>
        </div>
        ${source.last_error ? `<div class="err">${esc(source.last_error)}</div>` : ""}
        <div class="help">${esc(help[0].toUpperCase() + help.slice(1))}</div>
      </div>`;
  }

  /** Menu bar and login options. */
  appHtml() {
    const s = this.store.settings;
    return `
      <div class="section">
        <h4>App</h4>
        <div class="card">
          <div class="field">
            <div class="lbl">Appearance</div>
            <div class="ctrl">${Sheet.segmented("appearance", s.appearance || "system", [["system", "System"], ["light", "Light"], ["dark", "Dark grey"]])}</div>
          </div>
          <div class="field inline">
            <div><div class="lbl">Keep running in the ${Platform.trayName}</div><div class="help">Closing the window keeps checks going in the background.</div></div>
            <div class="ctrl">${Sheet.toggle("run_in_background", s.run_in_background)}</div>
          </div>
          <div class="field inline">
            <div class="lbl">Open at login</div>
            <div class="ctrl">${Sheet.toggle("launch_at_login", s.launch_at_login)}</div>
          </div>
        </div>
      </div>`;
  }

  /** "Every hour", "Every 3 hours", "Once a day"… */
  static intervalLabel(minutes) {
    if (minutes < 60) return `Every ${minutes} minutes`;
    if (minutes === 60) return "Every hour";
    if (minutes === 1440) return "Once a day";
    return `Every ${minutes / 60} hours`;
  }

  // ---------------------------------------------------------------- input

  /** Buttons and links inside the panel. */
  async onClick(event) {
    const t = event.target.closest("button, a");
    if (!t) return;
    if (t.hasAttribute("data-close")) return this.close();
    if (t.hasAttribute("data-about")) return this.actions.openAbout();
    if (t.dataset.url) return this.call(() => Api.openUrl(t.dataset.url));

    const segment = t.parentElement?.dataset.seg;
    if (segment && t.dataset.v) return this.save(segment, t.dataset.v);

    if (t.hasAttribute("data-choose-base")) return this.chooseBaseFolder();
    if (t.hasAttribute("data-apply-defaults")) {
      return Sheet.confirm(t, "Click again: change all repositories", () => this.applyDefaultsToAll());
    }
    if (t.hasAttribute("data-token-save")) return this.saveToken();
    if (t.hasAttribute("data-token-remove")) return this.removeToken();
    if (t.dataset.unfollow) return Sheet.confirm(t, "Sure? Files are kept", () => this.unfollow(t.dataset.unfollow));
    if (t.dataset.choose) return this.actions.chooseRepos(t.dataset.choose);
  }

  /** Switches and menus. */
  onChange(event) {
    const t = event.target;
    if (t.dataset.sw) this.save(t.dataset.sw, t.checked);
    if (t.dataset.number) this.save(t.dataset.number, Number(t.value));
  }

  // ---------------------------------------------------------------- actions

  /** Saves one setting; `key` may be nested, e.g. "defaults.format". */
  async save(key, value) {
    const settings = JSON.parse(JSON.stringify(this.store.settings));
    const [first, second] = key.split(".");
    if (second) settings[first][second] = value;
    else settings[first] = value;
    const { ok } = await this.call(() => Api.setSettings(settings));
    if (ok) this.store.app.settings = settings;
    this.render();
  }

  /** Lets the user pick a new main folder. */
  async chooseBaseFolder() {
    const { value: picked } = await this.call(() => Api.pickFolder(this.store.settings.base_folder));
    if (!picked) return;
    await this.save("base_folder", picked);
    this.actions.reload();
  }

  /** Gives all existing repositories the defaults above. */
  async applyDefaultsToAll() {
    const { ok, value: changed } = await this.call(() => Api.applyDefaultsToAll());
    if (!ok) return;
    this.toast.show(changed
      ? `${Format.repositories(changed)} changed. Press “Update all” to apply.`
      : "All repositories already use these settings.");
  }

  /** Saves the token typed in the field. */
  async saveToken() {
    const token = this.el.querySelector("#token-input").value.trim();
    if (!token) return this.toast.error("Paste a token first.");
    const { ok, value: status } = await this.call(() => Api.setToken(token));
    if (!ok) return;
    this.tokenStatus = status;
    this.render();
    if (status.error) this.toast.error(status.error);
    else this.toast.show(`Token saved in ${Platform.keychain}`);
  }

  /** Removes the saved token. */
  async removeToken() {
    const { ok, value: status } = await this.call(() => Api.setToken(null));
    if (!ok) return;
    this.tokenStatus = status;
    this.render();
    this.toast.show("Token removed");
  }

  /** Unfollows an account (files stay). */
  async unfollow(id) {
    const { ok } = await this.call(() => Api.removeSource(id));
    if (ok) this.toast.show("Unfollowed. Files are kept.");
  }

  /** Runs an API call; shows an error instead of throwing. Returns { ok, value }. */
  async call(work) {
    try {
      return { ok: true, value: await work() };
    } catch (error) {
      this.toast.error(error);
      return { ok: false, value: null };
    }
  }
}

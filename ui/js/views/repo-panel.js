// Side panel with one repository's details and settings.

import { Api } from "../api.js";
import { Format } from "../format.js";
import { Icons } from "../icons.js";
import { Sheet } from "./sheet.js";
import { Progress } from "./progress.js";
import { Platform } from "../platform.js";

const esc = Format.esc;

export class RepoPanel extends Sheet {
  /** `actions`: { updateRepo(id), selectionChanged(id) } */
  constructor(store, toast, actions) {
    super(document.querySelector("#panel"));
    this.store = store;
    this.toast = toast;
    this.actions = actions;
    this.repoId = null;
    this.el.addEventListener("click", e => this.onClick(e));
    this.el.addEventListener("change", e => this.onChange(e));
    this.el.addEventListener("keydown", e => this.onKeyDown(e));
    this.el.addEventListener("focusout", e => this.onFocusOut(e));
  }

  /** Opens the panel for one repository. */
  show(id) {
    this.repoId = id;
    this.open();
    this.render(true);
    this.actions.selectionChanged(id);
  }

  /** Forgets the repository when the panel closes. */
  onClosed() {
    this.repoId = null;
    this.actions.selectionChanged(null);
  }

  /** The repository shown. */
  get repo() {
    return this.store.repos.get(this.repoId);
  }

  // ---------------------------------------------------------------- drawing

  /** Redraws only when something besides progress changed; never while the user is
   *  typing in a field. Progress itself is written straight into the status box. */
  render(force = false) {
    if (!this.isOpen) return;
    const repo = this.repo;
    if (!repo) return this.close();

    const focused = document.activeElement;
    const typing = focused && this.el.contains(focused) && focused.matches("input.text, select");
    if (force || !typing) this.renderHtml(repo, force);
    Progress.apply(this.el, repo);
  }

  /** Builds the panel's HTML (redrawn only if it differs from what's shown). */
  renderHtml(repo, force) {
    this.renderKeepingScroll(
      this.headHtml(repo) +
        `<div class="sheet-body">
          ${this.statusHtml(repo)}
          ${this.locationHtml(repo)}
          ${this.updatesHtml(repo)}
          ${this.storageHtml(repo)}
          ${this.versionsHtml(repo)}
          ${this.detailsHtml(repo)}
          ${this.removeHtml(repo)}
        </div>`,
      force
    );
  }

  /** Name, owner, close button. */
  headHtml(repo) {
    return `
      <div class="sheet-head">
        <div class="mark">${Icons.mark(repo.status, repo.busy)}</div>
        <div class="t">
          <h3 class="selectable">${esc(repo.name)}</h3>
          <div class="owner">${esc(repo.owner)}${repo.description ? " · " + esc(repo.description) : ""}</div>
        </div>
        ${Sheet.closeButton(Icons.close)}
      </div>`;
  }

  /** Coloured status box (text and bar filled live) and the three main buttons. */
  statusHtml(repo) {
    const bar = repo.busy?.phase === "downloading" ? Progress.barHtml : "";
    return `
      <div class="statusbox s-${repo.status}"><span data-live></span>${bar}</div>
      <div class="btnrow">
        <button class="btn primary" data-update="${esc(repo.id)}" ${repo.can_update && !repo.busy ? "" : "disabled"}>Update now</button>
        <button class="btn" data-reveal="${esc(repo.local?.path || repo.destination)}">Show in ${Platform.fileManager}</button>
        <button class="btn" data-url="${esc(repo.html_url)}">Open on GitHub</button>
      </div>`;
  }

  /** Destination folder. */
  locationHtml(repo) {
    const next = repo.target_hint.split("/").slice(-2).join("/");
    return `
      <div class="section">
        <h4>Location</h4>
        <div class="card">
          <div class="field">
            <div class="lbl">Folder</div>
            <div class="ctrl">
              <div class="path selectable" title="${esc(repo.destination)}">${Format.path(repo.destination)}</div>
              <button class="btn small" data-choose>Choose…</button>
              ${repo.settings.destination ? `<button class="btn small" data-default-dest>Default</button>` : ""}
            </div>
            <div class="help">Next copy: <span class="mono">${esc(next)}</span></div>
          </div>
        </div>
      </div>`;
  }

  /** Auto-update and branch. */
  updatesHtml(repo) {
    return `
      <div class="section">
        <h4>Updates</h4>
        <div class="card">
          <div class="field inline">
            <div><div class="lbl">Update automatically</div><div class="help">Downloads new versions after each check.</div></div>
            <div class="ctrl">${Sheet.toggle("auto_update", repo.settings.auto_update)}</div>
          </div>
          <div class="field">
            <div class="lbl">Branch</div>
            <div class="ctrl"><input class="text mono" data-branch value="${esc(repo.settings.branch || "")}" placeholder="${esc(repo.default_branch)} (default)" spellcheck="false"></div>
          </div>
        </div>
      </div>`;
  }

  /** Zip or folder, replace or keep versions. */
  storageHtml(repo) {
    const s = repo.settings;
    const keepOptions = [0, 1, 2, 3, 5, 10, 20, 50]
      .map(n => `<option value="${n}" ${s.keep_versions === n ? "selected" : ""}>${n === 0 ? "All versions" : `Last ${n}`}</option>`)
      .join("");
    return `
      <div class="section">
        <h4>Storage</h4>
        <div class="card">
          <div class="field">
            <div class="lbl">Keep as</div>
            <div class="ctrl">${Sheet.segmented("format", s.format, [["zip", "Zip archive"], ["folder", "Unpacked folder"]])}</div>
          </div>
          <div class="field">
            <div class="lbl">When a new version arrives</div>
            <div class="ctrl">${Sheet.segmented("mode", s.mode, [["replace", "Replace the old copy"], ["keep_versions", "Keep every version"]])}</div>
            ${s.mode === "keep_versions" ? `<div class="ctrl"><span class="muted">Keep</span><select class="text" data-keep>${keepOptions}</select></div>` : ""}
            ${this.modeHelp(repo)}
          </div>
        </div>
      </div>`;
  }

  /** Explains what the chosen storage settings do. */
  modeHelp(repo) {
    const s = repo.settings;
    if (s.mode === "keep_versions") {
      const limit = s.keep_versions ? ` Versions older than the last ${s.keep_versions} are removed.` : "";
      return `<div class="help">Each version is saved next to the earlier ones, in “${esc(repo.name)} versions”.${limit}</div>`;
    }
    return s.format === "folder"
      ? `<div class="help warn">The folder is replaced on each update, so any changes you make inside it are overwritten.</div>`
      : `<div class="help">The zip is swapped for the new one once the download is complete and verified.</div>`;
  }

  /** Kept versions, newest first. */
  versionsHtml(repo) {
    if (!repo.versions.length) return "";
    const rows = [...repo.versions].reverse().map(v => `
      <div class="v">
        <span class="mono">${Format.sha(v.sha)}</span>
        <span class="when">${esc(Format.when(v.downloaded_at))}</span>
        <span class="muted">${Format.bytes(v.size_bytes)}</span>
        <button class="btn small" data-reveal="${esc(v.path)}">Show</button>
      </div>`).join("");
    return `<div class="section"><h4>Versions on this computer</h4><div class="card versions">${rows}</div></div>`;
  }

  /** Commit ids, path, size, source. */
  detailsHtml(repo) {
    const local = repo.local;
    const source = repo.source ? `Followed account <b>${esc(repo.owner)}</b>` : "Added on its own";
    return `
      <div class="section">
        <h4>Details</h4>
        <div class="card">
          <dl class="kv selectable">
            <dt>Latest on GitHub</dt><dd><span class="mono">${Format.sha(repo.remote_sha)}</span> on <span class="mono">${esc(repo.branch)}</span>${repo.remote_checked_at ? ` · checked ${Format.ago(repo.remote_checked_at)}` : ""}</dd>
            <dt>On this computer</dt><dd>${local ? `<span class="mono">${Format.sha(local.sha)}</span> · ${esc(Format.when(local.downloaded_at))}` : "Not downloaded yet"}</dd>
            ${local ? `<dt>Size</dt><dd>${Format.bytes(local.size_bytes)}</dd><dt>Path</dt><dd class="mono">${esc(local.path)}</dd>` : ""}
            ${repo.gone_since ? `<dt>Gone since</dt><dd>${esc(Format.when(repo.gone_since))}</dd>` : ""}
            <dt>Source</dt><dd>${source}</dd>
          </dl>
        </div>
      </div>`;
  }

  /** "Stop tracking" at the bottom. */
  removeHtml(repo) {
    const again = repo.source ? ` and won't add it again from ${esc(repo.owner)}` : "";
    return `
      <div class="danger-zone">
        <button class="btn danger" data-remove>Stop tracking</button>
        <p>Repopus stops checking this repository${again}. Files on this computer are kept.</p>
      </div>`;
  }

  // ---------------------------------------------------------------- input

  /** Buttons inside the panel. */
  async onClick(event) {
    const button = event.target.closest("button");
    if (!button) return;
    if (button.hasAttribute("data-close")) return this.close();
    if (button.dataset.update) return this.actions.updateRepo(button.dataset.update);

    const segment = button.parentElement?.dataset.seg;
    if (segment && button.dataset.v) return this.save({ [segment]: button.dataset.v });

    if (button.hasAttribute("data-choose")) {
      const { value: picked } = await this.call(() => Api.pickFolder(this.repo?.destination));
      if (picked) this.save({ destination: picked });
      return;
    }
    if (button.hasAttribute("data-default-dest")) return this.save({ destination: null });
    if (button.hasAttribute("data-remove")) {
      return Sheet.confirm(button, "Click again to stop tracking", () => this.stopTracking());
    }
  }

  /** Switches and the "keep" menu. */
  onChange(event) {
    const t = event.target;
    if (t.dataset.sw) this.save({ [t.dataset.sw]: t.checked });
    if (t.hasAttribute("data-keep")) this.save({ keep_versions: Number(t.value) });
  }

  /** Enter in the branch field saves it. */
  onKeyDown(event) {
    if (event.target.matches("[data-branch]") && event.key === "Enter") {
      event.preventDefault();
      event.target.blur();
    }
  }

  /** Leaving the branch field saves it. */
  onFocusOut(event) {
    if (!event.target.matches("[data-branch]") || !this.repo) return;
    const branch = event.target.value.trim() || null;
    if (branch !== (this.repo.settings.branch || null)) this.save({ branch });
  }

  // ---------------------------------------------------------------- actions

  /** Saves changed settings and redraws. */
  async save(patch) {
    const repo = this.repo;
    if (!repo) return;
    const { ok, value: view } = await this.call(() => Api.setRepoSettings(repo.id, { ...repo.settings, ...patch }));
    if (!ok) return this.render(true); // put the controls back as they were
    this.store.putRepo(view);
    this.render(true);
  }

  /** Removes the repository from Repopus (files stay). */
  async stopTracking() {
    const id = this.repoId;
    const { ok } = await this.call(() => Api.removeRepo(id));
    if (!ok) return;
    this.close();
    this.toast.show("Stopped tracking. Files are kept.");
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

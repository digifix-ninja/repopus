// Pop-up for choosing which repositories of an account Repopus tracks. Opens when an
// account link is added, and again from the account's category or the settings.

import { Api } from "../api.js";
import { Format } from "../format.js";
import { Icons } from "../icons.js";
import { Sheet } from "./sheet.js";

const esc = Format.esc;

export class RepoPicker extends Sheet {
  constructor(toast) {
    super(document.querySelector("#picker"));
    this.toast = toast;
    this.account = null;    // AccountChoice from Rust
    this.chosen = new Set(); // repository names switched on
    this.addNew = true;
    this.query = "";
    this.saving = false;
    this.el.addEventListener("click", e => this.onClick(e));
    this.el.addEventListener("change", e => this.onChange(e));
    this.el.addEventListener("input", e => this.onInput(e));
  }

  /** Opens with an account's repositories already fetched (after adding a link). */
  show(account) {
    this.account = account;
    this.chosen = new Set(account.repos.filter(r => r.selected).map(r => r.name));
    this.addNew = account.add_new;
    this.query = "";
    this.saving = false;
    this.open();
    this.renderAll();
    this.el.querySelector("#picker-filter")?.focus();
  }

  /** Opens for a followed account, fetching its repositories from GitHub first. */
  async showFor(owner) {
    this.account = null;
    this.open();
    this.el.innerHTML = `
      ${this.headHtml(owner, "Asking GitHub for the repositories…")}
      <div class="sheet-body"><div class="nothing"><div class="spinner"></div></div></div>`;
    try {
      const account = await Api.accountRepos(owner);
      if (this.isOpen) this.show(account);
    } catch (error) {
      this.close();
      this.toast.error(error);
    }
  }

  /** Forgets the account when the pop-up closes. */
  onClosed() {
    this.account = null;
  }

  // ---------------------------------------------------------------- drawing

  /** Draws the whole pop-up (once per opening; later changes redraw only parts). */
  renderAll() {
    const a = this.account;
    const kind = a.account_type === "Organization" ? "Organisation" : "User";
    this.el.innerHTML = `
      ${this.headHtml(a.owner, `${kind} · ${Format.repositories(a.repos.length)} on GitHub`)}
      <div class="picker-tools">
        <input class="text" id="picker-filter" placeholder="Filter" spellcheck="false">
        <button class="link-btn" data-all>Select all</button>
        <span class="dot">·</span>
        <button class="link-btn" data-none>Select none</button>
      </div>
      <div class="sheet-body picker-list"></div>
      <div class="picker-foot">
        <div class="field inline">
          <div><div class="lbl">Add new repositories automatically</div>
            <div class="help">Repositories ${esc(a.owner)} creates later are added on their own.</div></div>
          <div class="ctrl">${Sheet.toggle("add_new", this.addNew)}</div>
        </div>
        <div class="picker-actions">
          <span class="picker-note"></span>
          <button class="btn" data-close>Cancel</button>
          <button class="btn primary" data-save></button>
        </div>
      </div>`;
    this.renderList();
    this.renderFoot();
  }

  /** Title, subtitle and close button. */
  headHtml(owner, subtitle) {
    return `
      <div class="sheet-head">
        <div class="t"><h3>Choose repositories from ${esc(owner)}</h3><div class="owner">${esc(subtitle)}</div></div>
        ${Sheet.closeButton(Icons.close)}
      </div>`;
  }

  /** The repositories matching the filter. */
  get visible() {
    const q = this.query.toLowerCase();
    if (!q) return this.account.repos;
    return this.account.repos.filter(r => [r.name, r.description || ""].some(t => t.toLowerCase().includes(q)));
  }

  /** The list of repositories with their switches. */
  renderList() {
    const rows = this.visible.map(r => `
      <label class="pick">
        <div class="main">
          <div class="title"><span class="name">${esc(r.name)}</span>${RepoPicker.tagsHtml(r)}</div>
          <div class="desc">${esc(r.description || "No description")}</div>
        </div>
        <div class="when">${r.pushed_at ? esc(Format.ago(r.pushed_at)) : ""}</div>
        <span class="switch"><input type="checkbox" data-repo="${esc(r.name)}" ${this.chosen.has(r.name) ? "checked" : ""}><span></span></span>
      </label>`).join("");
    const empty = this.account.repos.length ? "No matches." : "This account has no repositories yet.";
    this.el.querySelector(".picker-list").innerHTML = rows || `<div class="nothing">${empty}</div>`;
  }

  /** Save button text, and what will stop being tracked. */
  renderFoot() {
    const a = this.account;
    const count = this.chosen.size;
    const button = this.el.querySelector("[data-save]");
    button.textContent = this.saving ? "Saving…" : a.following ? "Save" : `Follow with ${Format.repositories(count)}`;
    button.disabled = this.saving || (!a.following && count === 0 && !this.addNew);

    const dropped = a.repos.filter(r => r.selected && !this.chosen.has(r.name)).length;
    this.el.querySelector(".picker-note").textContent = a.following && dropped
      ? `${Format.repositories(dropped)} will no longer be tracked. Files are kept.`
      : `${count} of ${a.repos.length} selected`;
  }

  /** Small labels after the name: private, fork, archived. */
  static tagsHtml(repo) {
    const tags = [];
    if (repo.private) tags.push(`<span class="tag">private</span>`);
    if (repo.fork) tags.push(`<span class="tag">fork</span>`);
    if (repo.archived) tags.push(`<span class="tag">archived</span>`);
    return tags.join("");
  }

  // ---------------------------------------------------------------- input

  /** Buttons inside the pop-up. */
  onClick(event) {
    const button = event.target.closest("button");
    if (!button) return;
    if (button.hasAttribute("data-close")) return this.close();
    if (!this.account) return;
    if (button.hasAttribute("data-all")) return this.setVisible(true);
    if (button.hasAttribute("data-none")) return this.setVisible(false);
    if (button.hasAttribute("data-save")) return this.save();
  }

  /** Repository switches and "add new automatically". */
  onChange(event) {
    const t = event.target;
    if (t.dataset.repo) {
      if (t.checked) this.chosen.add(t.dataset.repo);
      else this.chosen.delete(t.dataset.repo);
    }
    if (t.dataset.sw === "add_new") this.addNew = t.checked;
    this.renderFoot();
  }

  /** Typing in the filter. */
  onInput(event) {
    if (event.target.id !== "picker-filter") return;
    this.query = event.target.value.trim();
    this.renderList();
  }

  // ---------------------------------------------------------------- actions

  /** Switches every repository matching the filter on or off. */
  setVisible(on) {
    for (const r of this.visible) {
      if (on) this.chosen.add(r.name);
      else this.chosen.delete(r.name);
    }
    this.renderList();
    this.renderFoot();
  }

  /** Follows the account with the chosen repositories (or saves the new choice). */
  async save() {
    this.saving = true;
    this.renderFoot();
    try {
      const message = await Api.chooseRepos(this.account.owner, [...this.chosen], this.addNew);
      this.close();
      this.toast.show(message);
    } catch (error) {
      this.toast.error(error);
      this.saving = false;
      if (this.account) this.renderFoot();
    }
  }
}

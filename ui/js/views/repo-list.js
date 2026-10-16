// The main list: one foldable category per owner, one row per repository.
//
// The list is updated in place: a row is only rebuilt when its content really changes.
// Progress (bar width, "3.2 MB of 14 MB") is written straight into the existing row,
// so hover states and spinners keep running smoothly during downloads.

import { Format } from "../format.js";
import { Icons } from "../icons.js";
import { Progress } from "./progress.js";

const esc = Format.esc;

export class RepoList {
  /** `actions`: { openRepo(id), updateRepo(id), toggleGroup(key), updateGroup(owner), fillExample(text) } */
  constructor(store, actions) {
    this.store = store;
    this.actions = actions;
    this.el = document.querySelector("#list");
    this.selectedId = null;
    this.groupParts = new Map(); // key -> { section, head, rows, headHtml, rowsNote }
    this.rowParts = new Map();   // id  -> { el, html }
    this.staticHtml = null;      // shown instead of groups (first launch, no matches)
    this.el.addEventListener("click", e => this.onClick(e));
  }

  /** Marks the row whose panel is open. */
  select(id) {
    this.selectedId = id;
  }

  /** Brings the list up to date, touching only what changed. */
  render() {
    if (!this.store.app) return;
    if (this.store.isEmpty) return this.showStatic(this.emptyHtml());
    const groups = this.store.groups().filter(g => g.visible.length || !this.store.narrowed);
    if (!groups.length) return this.showStatic(this.nothingHtml());
    this.showGroups(groups);
  }

  // ---------------------------------------------------------------- in-place updating

  /** Replaces the list with fixed content (only when it differs). */
  showStatic(html) {
    if (this.staticHtml === html) return;
    this.staticHtml = html;
    this.groupParts.clear();
    this.rowParts.clear();
    this.el.innerHTML = html;
  }

  /** Shows the categories in order, reusing their existing elements. */
  showGroups(groups) {
    if (this.staticHtml !== null) {
      this.staticHtml = null;
      this.el.innerHTML = "";
    }
    const seenGroups = new Set();
    const seenRows = new Set();
    let previous = null;
    for (const group of groups) {
      const section = this.syncGroup(group, seenRows);
      RepoList.placeAfter(this.el, section, previous);
      previous = section;
      seenGroups.add(group.key);
    }
    RepoList.removeAfter(this.el, previous);
    RepoList.forgetUnseen(this.groupParts, seenGroups);
    RepoList.forgetUnseen(this.rowParts, seenRows);
  }

  /** Updates one category: header, folded state and rows. */
  syncGroup(group, seenRows) {
    let parts = this.groupParts.get(group.key);
    if (!parts) {
      parts = RepoList.createGroupParts(group.key);
      this.groupParts.set(group.key, parts);
    }
    parts.section.classList.toggle("collapsed", group.collapsed);
    parts.head.title = `${group.collapsed ? "Open" : "Fold"} ${group.title}`;

    const headHtml = this.headHtml(group);
    if (parts.headHtml !== headHtml) {
      parts.head.innerHTML = headHtml;
      parts.headHtml = headHtml;
    }

    if (group.collapsed) {
      parts.rows.remove();
      return parts.section;
    }
    if (!parts.rows.isConnected) parts.section.append(parts.rows);

    if (!group.visible.length) {
      parts.rows.innerHTML = `<div class="nothing">No repositories yet. They appear after the first check.</div>`;
      return parts.section;
    }
    let previous = null;
    for (const repo of group.visible) {
      const row = this.syncRow(repo);
      RepoList.placeAfter(parts.rows, row, previous);
      previous = row;
      seenRows.add(repo.id);
    }
    RepoList.removeAfter(parts.rows, previous);
    return parts.section;
  }

  /** Returns the row element for a repository, rebuilt only if its HTML changed. */
  syncRow(repo) {
    const html = this.rowHtml(repo);
    let parts = this.rowParts.get(repo.id);
    if (!parts || parts.html !== html) {
      const el = RepoList.elementFrom(html);
      if (parts) parts.el.replaceWith(el);
      parts = { el, html };
      this.rowParts.set(repo.id, parts);
    }
    Progress.apply(parts.el, repo);
    return parts.el;
  }

  /** Empty section, header and rows container for a new category. */
  static createGroupParts(key) {
    const section = document.createElement("section");
    section.className = "group";
    const head = document.createElement("div");
    head.className = "group-head";
    head.dataset.group = key;
    const rows = document.createElement("div");
    rows.className = "rows";
    section.append(head, rows);
    return { section, head, rows, headHtml: "" };
  }

  /** Puts `el` right after `previous` (or first) — only moves it if it's elsewhere,
   *  because moving an element restarts its animations. */
  static placeAfter(parent, el, previous) {
    const expected = previous ? previous.nextSibling : parent.firstChild;
    if (expected !== el) parent.insertBefore(el, expected);
  }

  /** Removes everything after `last` (everything, if `last` is null). */
  static removeAfter(parent, last) {
    while (last ? last.nextSibling : parent.firstChild) {
      (last ? last.nextSibling : parent.firstChild).remove();
    }
  }

  /** Drops cached parts that are no longer shown. */
  static forgetUnseen(map, seen) {
    for (const key of [...map.keys()]) if (!seen.has(key)) map.delete(key);
  }

  /** Turns an HTML string into one element. */
  static elementFrom(html) {
    const template = document.createElement("template");
    template.innerHTML = html.trim();
    return template.content.firstElementChild;
  }

  // ---------------------------------------------------------------- clicks

  /** One click handler for the whole list. */
  onClick(event) {
    const target = event.target;
    const example = target.closest("[data-example]");
    if (example) return this.actions.fillExample(example.dataset.example);

    const groupUpdate = target.closest("[data-update-group]");
    if (groupUpdate) {
      event.stopPropagation();
      return this.actions.updateGroup(groupUpdate.dataset.updateGroup);
    }
    const update = target.closest("[data-update]");
    if (update) {
      event.stopPropagation();
      return this.actions.updateRepo(update.dataset.update);
    }
    const head = target.closest(".group-head");
    if (head) return this.actions.toggleGroup(head.dataset.group);

    const row = target.closest(".row");
    if (row) this.actions.openRepo(row.dataset.id);
  }

  // ---------------------------------------------------------------- HTML

  /** Inside of a category header: chevron, name, counts, "Update N". */
  headHtml(group) {
    const note = (group.source ? "Following · " : "") + Format.repositories(group.repos.length);
    const flags = [];
    if (group.updates) flags.push(`<span class="pill orange small"><i></i>${group.updates} to update</span>`);
    if (group.missing) flags.push(`<span class="pill red small"><i></i>${group.missing} missing</span>`);
    const error = group.source?.last_error ? `<span class="note err">${esc(group.source.last_error)}</span>` : "";
    const updateButton = group.updatable
      ? `<button class="btn small" data-update-group="${esc(group.title)}" title="Update everything in ${esc(group.title)}">${Icons.download}Update ${group.updatable}</button>`
      : "";
    return `
      <span class="fold">${Icons.chevron}</span>
      <h2>${esc(group.title)}</h2>
      <span class="note">${note}</span>
      ${flags.join("")}
      ${error}
      <span class="spacer"></span>
      ${updateButton}`;
  }

  /** One repository row. Changing numbers (progress) are left out; `Progress.apply` fills them. */
  rowHtml(repo) {
    const busy = repo.busy;
    const canUpdate = repo.can_update && !busy;
    const state = busy
      ? `<div class="state" data-live></div>`
      : `<div class="state">${esc(repo.status_text)}</div>`;
    const below = busy?.phase === "downloading"
      ? Progress.barHtml
      : `<div class="sub">${esc(busy?.phase === "waiting" ? "In the queue" : this.subline(repo))}</div>`;
    const busyClass = busy ? ` busy-${busy.phase}` : "";

    return `
      <div class="row s-${repo.status}${busyClass}${this.selectedId === repo.id ? " selected" : ""}" data-id="${esc(repo.id)}">
        <div class="mark">${Icons.mark(repo.status, busy)}</div>
        <div class="main">
          <div class="title"><span class="name">${esc(repo.name)}</span>${this.tagsHtml(repo)}</div>
          <div class="desc">${esc(repo.description || "No description")}</div>
        </div>
        <div class="meta">${state}${below}</div>
        <div class="auto${repo.settings.auto_update ? " on" : ""}" title="${repo.settings.auto_update ? "Updates automatically" : "Manual updates only"}">${Icons.auto}</div>
        <button class="btn small" data-update="${esc(repo.id)}" ${canUpdate ? "" : "disabled"}>Update</button>
        <div class="chev">›</div>
      </div>`;
  }

  /** Small labels after the name: private, fork, archived, non-default branch. */
  tagsHtml(repo) {
    const tags = [];
    if (repo.private) tags.push(`<span class="tag">private</span>`);
    if (repo.fork) tags.push(`<span class="tag">fork</span>`);
    if (repo.archived) tags.push(`<span class="tag">archived</span>`);
    if (repo.branch !== repo.default_branch) tags.push(`<span class="tag branch">${esc(repo.branch)}</span>`);
    return tags.join("");
  }

  /** "2 h ago · 14 MB · zip" under the status text. */
  subline(repo) {
    if (repo.local) {
      const parts = [Format.ago(repo.local.downloaded_at), Format.bytes(repo.local.size_bytes), repo.local.format === "zip" ? "zip" : "folder"];
      if (repo.settings.mode === "keep_versions" && repo.versions.length) parts.push(`${repo.versions.length} versions`);
      return parts.join(" · ");
    }
    return repo.remote_size_kb ? `about ${Format.bytes(repo.remote_size_kb * 1000)}` : "";
  }

  /** First launch: nothing added yet. */
  emptyHtml() {
    return `
      <div class="empty">
        <img src="logo.svg" alt="">
        <h1>Keep GitHub repositories on your computer</h1>
        <p>Paste a link above. An account link follows every repository it has, including new ones;
           a repository link adds just that one. Each account gets its own category.
           Nothing is ever deleted when something disappears from GitHub.</p>
        <div class="examples">
          <button class="btn small" data-example="github.com/">github.com/username</button>
          <button class="btn small" data-example="github.com/">github.com/username/repository</button>
        </div>
      </div>`;
  }

  /** Filter or search left nothing. */
  nothingHtml() {
    const text = { updates: "Nothing to update. 🎉", missing: "Nothing is missing." }[this.store.filter] || "No matches.";
    return `<div class="nothing">${text}</div>`;
  }
}

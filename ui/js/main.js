// Repopus window: creates the views and connects them to the Rust side.

import { Api } from "./api.js";
import { Platform } from "./platform.js";
import { Theme } from "./theme.js";
import { AppStore } from "./store.js";
import { Toast } from "./toast.js";
import { Header } from "./views/header.js";
import { RepoList } from "./views/repo-list.js";
import { RepoPanel } from "./views/repo-panel.js";
import { SettingsPanel } from "./views/settings-panel.js";
import { AboutPanel } from "./views/about-panel.js";
import { Sheet } from "./views/sheet.js";

class App {
  constructor() {
    this.store = new AppStore();
    this.toast = new Toast(document.querySelector("#toast"));
    this.header = new Header(this.store);
    this.list = new RepoList(this.store, {
      openRepo: id => this.panel.show(id),
      updateRepo: id => this.updateRepo(id),
      toggleGroup: key => this.toggleGroup(key),
      updateGroup: owner => this.run(() => Api.updateOwner(owner)),
      fillExample: text => this.fillAddField(text),
    });
    this.panel = new RepoPanel(this.store, this.toast, {
      updateRepo: id => this.updateRepo(id),
      selectionChanged: id => { this.list.select(id); this.store.changed(); },
    });
    this.settings = new SettingsPanel(this.store, this.toast, {
      reload: () => this.reload(),
      openAbout: () => this.about.show(),
    });
    this.about = new AboutPanel(this.toast);
    this.store.subscribe(() => this.render());
  }

  /** Loads the state, then listens for changes. */
  async start() {
    Platform.markBody();
    this.bindControls();
    this.bindShortcuts();
    this.listenToRust();
    await this.reload();
    setInterval(() => this.store.changed(), 30_000); // keeps "5 min ago" fresh
  }

  /** Redraws everything that depends on the store. */
  render() {
    Theme.apply(this.store.settings?.appearance);
    this.header.render();
    this.list.render();
    this.panel.render();
    this.settings.render();
  }

  /** Fetches the full state from Rust. */
  async reload() {
    try {
      this.store.load(await Api.getState());
    } catch (error) {
      this.toast.error(error);
    }
  }

  // ---------------------------------------------------------------- actions

  /** Runs an API call and shows its error, if any. */
  async run(work) {
    try {
      return await work();
    } catch (error) {
      this.toast.error(error);
    }
  }

  /** Starts updating one repository (the row turns busy at once). */
  updateRepo(id) {
    this.store.setProgress(id, { phase: "working", text: "Preparing…", bytes: 0, total: null });
    this.run(() => Api.updateRepo(id));
  }

  /** Folds or opens a category and remembers it. */
  toggleGroup(key) {
    const collapsed = this.store.toggleGroup(key);
    this.run(() => Api.setGroupCollapsed(key, collapsed));
  }

  /** Folds or opens all categories and remembers it. */
  setAllCollapsed(collapsed) {
    this.store.setAllCollapsed(collapsed);
    this.run(() => Api.setAllCollapsed(collapsed));
  }

  /** Puts example text in the add field, ready to finish typing. */
  fillAddField(text) {
    const input = document.querySelector("#add-input");
    input.value = text;
    input.focus();
  }

  /** Adds the link typed in the add field. */
  async addLink() {
    const input = document.querySelector("#add-input");
    const button = document.querySelector("#add-btn");
    const link = input.value.trim();
    if (!link) return input.focus();
    button.disabled = true;
    button.textContent = "Adding…";
    try {
      this.toast.show(await Api.addLink(link));
      input.value = "";
    } catch (error) {
      this.toast.error(error);
    } finally {
      button.disabled = false;
      button.textContent = "Add";
    }
  }

  /** Starts a check now. */
  checkNow() {
    if (!this.store.app) return;
    this.store.setActivity({ checking: true, updating: this.store.app.updating, message: null });
    this.run(() => Api.checkNow());
  }

  // ---------------------------------------------------------------- wiring

  /** Buttons, fields and links in the main window. */
  bindControls() {
    const on = (selector, event, handler) => document.querySelector(selector).addEventListener(event, handler);

    on("#add-form", "submit", e => { e.preventDefault(); this.addLink(); });
    on("#btn-check", "click", () => this.checkNow());
    on("#btn-update-all", "click", () => this.run(() => Api.updateAll()));
    on("#btn-settings", "click", () => this.settings.toggleOpen());
    on("#btn-about", "click", () => (this.about.isOpen ? this.about.close() : this.about.show()));
    on("#btn-collapse-all", "click", () => this.setAllCollapsed(true));
    on("#btn-expand-all", "click", () => this.setAllCollapsed(false));
    on("#scrim", "click", () => Sheet.closeAll());
    on("#search", "input", e => { this.store.query = e.target.value.trim(); this.store.changed(); });
    on("#filters", "click", e => {
      const button = e.target.closest("button");
      if (button) { this.store.filter = button.dataset.filter; this.store.changed(); }
    });

    // "Show in Finder/Explorer" and "Open on GitHub" buttons anywhere outside the settings and about panels.
    document.addEventListener("click", e => {
      const t = e.target.closest("[data-reveal], [data-url]");
      if (!t || t.closest("#settings, #about")) return;
      if (t.dataset.reveal) this.run(() => Api.reveal(t.dataset.reveal));
      else this.run(() => Api.openUrl(t.dataset.url));
    });
  }

  /** Esc, ⌘R, ⌘, and ⌘F. */
  bindShortcuts() {
    document.addEventListener("keydown", e => {
      if (e.key === "Escape") return Sheet.closeAll();
      if (!e.metaKey) return;
      const key = e.key.toLowerCase();
      if (key === "r") { e.preventDefault(); this.checkNow(); }
      if (key === ",") { e.preventDefault(); this.settings.show(); }
      if (key === "f") { e.preventDefault(); document.querySelector("#search").focus(); }
    });
  }

  /** Events from the Rust side. */
  listenToRust() {
    Api.on("repo", view => this.store.putRepo(view));
    Api.on("repo-removed", id => {
      this.store.dropRepo(id);
      if (this.panel.repoId === id) this.panel.close();
    });
    Api.on("progress", ({ id, busy }) => this.store.setProgress(id, busy));
    Api.on("activity", activity => this.store.setActivity(activity));

    // Bursts of "store-changed" are merged into one reload.
    let timer = null;
    Api.on("store-changed", () => {
      clearTimeout(timer);
      timer = setTimeout(() => this.reload(), 120);
    });
  }
}

new App().start();

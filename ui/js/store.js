// What the window knows: the app state from Rust plus view choices (filter, search,
// folded categories). Views read from here; changes call `changed()`, which redraws once
// per frame.

export class AppStore {
  constructor() {
    this.app = null;              // AppView from Rust
    this.repos = new Map();       // id -> RepoView
    this.collapsed = new Set();   // lower-case owners folded shut
    this.filter = "all";          // "all" | "updates" | "missing"
    this.query = "";
    this.activity = { text: "", error: false, at: 0 };
    this.listeners = new Set();
    this.pending = false;
  }

  // ---------------------------------------------------------------- change notification

  /** Registers a function to run after changes. */
  subscribe(listener) {
    this.listeners.add(listener);
  }

  /** Asks for a redraw (several changes in one frame cause one redraw). */
  changed() {
    if (this.pending) return;
    this.pending = true;
    requestAnimationFrame(() => {
      this.pending = false;
      for (const listener of this.listeners) listener();
    });
  }

  // ---------------------------------------------------------------- updates from Rust

  /** Replaces everything with a fresh AppView. */
  load(view) {
    this.app = view;
    this.repos = new Map(view.repos.map(r => [r.id, r]));
    this.collapsed = new Set(view.collapsed_groups);
    this.changed();
  }

  /** One repository's new row. */
  putRepo(view) {
    this.repos.set(view.id, view);
    this.changed();
  }

  /** A repository is no longer tracked. */
  dropRepo(id) {
    this.repos.delete(id);
    this.changed();
  }

  /** Progress of a busy row ({ phase, text, bytes, total }). */
  setProgress(id, busy) {
    const repo = this.repos.get(id);
    if (!repo) return;
    Object.assign(repo, { busy, status: "busy", can_update: false });
    this.changed();
  }

  /** What the app as a whole is doing. */
  setActivity({ checking, updating, message, error }) {
    if (!this.app) return;
    this.app.checking = checking;
    this.app.updating = updating;
    if (message) this.activity = { text: message, error, at: Date.now() };
    else if (checking) this.activity = { text: "Checking…", error: false, at: Date.now() };
    this.changed();
  }

  // ---------------------------------------------------------------- reading

  get settings() { return this.app?.settings; }
  get sources() { return this.app?.sources ?? []; }
  get isEmpty() { return this.repos.size === 0 && this.sources.length === 0; }

  /** All rows. */
  all() {
    return [...this.repos.values()];
  }

  /** Whether a row shows a red mark. */
  static isMissing(repo) {
    return repo.status === "missing" || repo.status === "error";
  }

  /** Totals for the title bar and filters. */
  counts() {
    const all = this.all();
    return {
      all: all.length,
      upToDate: all.filter(r => r.status === "up_to_date").length,
      updates: all.filter(r => r.status === "update_available").length,
      missing: all.filter(AppStore.isMissing).length,
      updatable: all.filter(r => r.can_update).length,
    };
  }

  /** Whether a row passes the current filter and search. */
  matches(repo) {
    if (this.filter === "updates" && repo.status !== "update_available") return false;
    if (this.filter === "missing" && !AppStore.isMissing(repo)) return false;
    if (!this.query) return true;
    const q = this.query.toLowerCase();
    return [repo.name, repo.owner, repo.description || ""].some(t => t.toLowerCase().includes(q));
  }

  /** Whether a filter or search is narrowing the list. */
  get narrowed() {
    return this.filter !== "all" || this.query !== "";
  }

  /** Categories: one per owner. Followed accounts first (in the order they were added),
   *  then owners of repositories added on their own, alphabetically. */
  groups() {
    const groups = new Map();
    for (const source of this.sources) {
      groups.set(source.id, { key: source.id, title: source.owner, source, repos: [] });
    }
    const others = [];
    for (const repo of this.all()) {
      const key = repo.owner.toLowerCase();
      if (!groups.has(key)) {
        const group = { key, title: repo.owner, source: null, repos: [] };
        groups.set(key, group);
        others.push(group);
      }
      groups.get(key).repos.push(repo);
    }
    others.sort((a, b) => a.title.localeCompare(b.title));
    const followed = [...groups.values()].filter(g => g.source);

    return [...followed, ...others].map(group => {
      group.repos.sort((a, b) => a.name.localeCompare(b.name));
      group.visible = group.repos.filter(r => this.matches(r));
      group.updates = group.repos.filter(r => r.status === "update_available").length;
      group.missing = group.repos.filter(AppStore.isMissing).length;
      group.updatable = group.repos.filter(r => r.can_update).length;
      group.collapsed = this.isCollapsed(group.key);
      return group;
    });
  }

  // ---------------------------------------------------------------- folding

  /** Whether a category is folded. Searching opens everything, so matches show. */
  isCollapsed(key) {
    return !this.query && this.collapsed.has(key);
  }

  /** Folds or opens one category. Returns the new state. */
  toggleGroup(key) {
    const collapsed = !this.collapsed.has(key);
    if (collapsed) this.collapsed.add(key);
    else this.collapsed.delete(key);
    this.changed();
    return collapsed;
  }

  /** Folds (`true`) or opens (`false`) all categories. */
  setAllCollapsed(collapsed) {
    this.collapsed = collapsed ? new Set(this.groups().map(g => g.key)) : new Set();
    this.changed();
  }

  // ---------------------------------------------------------------- activity line

  /** Recent message (for 90 s, or while checking), otherwise "Last checked …". */
  activityLine(ago) {
    if (this.app?.updating) return { text: `Downloading ${this.app.updating}…`, error: false };
    const fresh = this.activity.text && (this.app?.checking || Date.now() - this.activity.at < 90_000);
    if (fresh) return { text: this.activity.text, error: this.activity.error };
    if (this.app?.last_check) return { text: `Last checked ${ago(this.app.last_check)}`, error: false };
    return { text: "", error: false };
  }
}

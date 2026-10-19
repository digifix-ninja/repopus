// Talking to the Rust side. Every command and event the window uses is listed here.

const tauri = window.__TAURI__;

export class Api {
  /** Calls a Rust command. */
  static call(command, args) {
    return tauri.core.invoke(command, args);
  }

  /** Runs `handler(payload)` whenever the Rust side sends `event`. */
  static on(event, handler) {
    return tauri.event.listen(event, ({ payload }) => handler(payload));
  }

  // ---- reading
  static getState() { return this.call("get_state"); }
  static tokenStatus() { return this.call("token_status"); }
  static about() { return this.call("about"); }
  static appUpdate() { return this.call("app_update"); }
  static checkAppUpdate() { return this.call("check_app_update"); }

  // ---- adding / removing
  static addLink(link) { return this.call("add_link", { link }); }
  static accountRepos(owner) { return this.call("account_repos", { owner }); }
  static chooseRepos(owner, chosen, addNew) { return this.call("choose_repos", { owner, chosen, addNew }); }
  static removeRepo(id) { return this.call("remove_repo", { id }); }
  static removeSource(id) { return this.call("remove_source", { id }); }

  // ---- checking / updating
  static checkNow() { return this.call("check_now"); }
  static updateRepo(id) { return this.call("update_repo", { id }); }
  static updateAll() { return this.call("update_all"); }
  static updateOwner(owner) { return this.call("update_owner", { owner }); }

  // ---- settings
  static setRepoSettings(id, settings) { return this.call("set_repo_settings", { id, settings }); }
  static setSettings(settings) { return this.call("set_settings", { settings }); }
  static applyDefaultsToAll() { return this.call("apply_defaults_to_all"); }
  static setGroupCollapsed(owner, collapsed) { return this.call("set_group_collapsed", { owner, collapsed }); }
  static setAllCollapsed(collapsed) { return this.call("set_all_collapsed", { collapsed }); }
  static setToken(token) { return this.call("set_token", { token }); }

  // ---- system helpers
  static pickFolder(start) { return this.call("pick_folder", { start }); }
  static reveal(path) { return this.call("reveal_path", { path }); }
  static openUrl(url) { return this.call("open_url", { url }); }
}

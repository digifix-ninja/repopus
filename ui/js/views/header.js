// Title bar and filter bar: summary pills, button states, filter counts, activity line.

import { Format } from "../format.js";

export class Header {
  constructor(store) {
    this.store = store;
    this.summary = document.querySelector("#summary");
    this.updateCount = document.querySelector("#update-count");
    this.updateAll = document.querySelector("#btn-update-all");
    this.check = document.querySelector("#btn-check");
    this.filters = document.querySelector("#filters");
    this.activity = document.querySelector("#activity");
  }

  /** Redraws everything in the bars. */
  render() {
    const counts = this.store.counts();
    this.renderSummary(counts);
    this.renderButtons(counts);
    this.renderFilters(counts);
    this.renderActivity();
  }

  /** Green / orange / red pills in the title bar. */
  renderSummary(counts) {
    const pills = [];
    if (counts.upToDate) pills.push(`<span class="pill green"><i></i>${counts.upToDate} up to date</span>`);
    if (counts.updates) pills.push(`<span class="pill orange"><i></i>${counts.updates} to update</span>`);
    if (counts.missing) pills.push(`<span class="pill red"><i></i>${counts.missing} missing</span>`);
    this.summary.innerHTML = pills.join("");
  }

  /** "Check now" spinner and the "Update all" count. */
  renderButtons(counts) {
    this.updateCount.textContent = counts.updatable || "";
    this.updateAll.disabled = counts.updatable === 0;
    const checking = !!this.store.app?.checking;
    this.check.classList.toggle("spinning", checking);
    this.check.disabled = checking;
    this.check.querySelector(".label").textContent = checking ? "Checking…" : "Check now";
  }

  /** Selected filter and its counts. */
  renderFilters(counts) {
    for (const button of this.filters.children) {
      button.classList.toggle("on", button.dataset.filter === this.store.filter);
      button.querySelector("span").textContent = counts[button.dataset.filter] || "";
    }
  }

  /** Text at the right of the filter bar. */
  renderActivity() {
    const line = this.store.activityLine(iso => Format.ago(iso));
    this.activity.textContent = line.text;
    this.activity.classList.toggle("error", line.error);
  }
}

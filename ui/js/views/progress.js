// Live progress inside an already drawn row or panel: only text and bar width change,
// so nothing is redrawn while a download runs.

import { Format } from "../format.js";

export class Progress {
  /** HTML of a progress bar (filled in by `apply`). */
  static get barHtml() {
    return `<div class="bar"><i></i></div>`;
  }

  /** Updates the `[data-live]` text and the `.bar` inside `root` for this repository. */
  static apply(root, repo) {
    const text = root.querySelector("[data-live]");
    if (text) {
      const value = repo.busy ? Format.busy(repo.busy) : repo.status_text;
      if (text.textContent !== value) text.textContent = value;
    }
    const bar = root.querySelector(".bar");
    if (!bar || !repo.busy) return;
    const fraction = Format.fraction(repo.busy);
    bar.classList.toggle("indeterminate", fraction === null);
    bar.firstElementChild.style.width = fraction === null ? "" : `${(fraction * 100).toFixed(1)}%`;
  }
}

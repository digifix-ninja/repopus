// Small inline SVG icons.

const EXCLAMATION = `<path d="M11 5.8v6.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/><circle cx="11" cy="16" r="1.5" fill="#fff"/>`;

export class Icons {
  /** The status mark at the start of a row (`busy` decides spinner or download badge). */
  static mark(status, busy = null) {
    if (status === "busy" && busy) {
      if (busy.phase === "downloading") {
        return `<svg viewBox="0 0 22 22"><circle cx="11" cy="11" r="10" fill="var(--accent)"/><path d="M11 6v8M7.6 10.8 11 14.2l3.4-3.4" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
      }
      return `<div class="spinner${busy.phase === "waiting" ? " waiting" : ""}"></div>`;
    }
    switch (status) {
      case "up_to_date":
        return `<svg viewBox="0 0 22 22"><circle cx="11" cy="11" r="10" fill="var(--green)"/><path d="m6.6 11.3 3 3 5.8-6.2" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
      case "update_available":
        return `<svg viewBox="0 0 22 22"><rect x="1" y="1" width="20" height="20" rx="6" fill="var(--orange)"/>${EXCLAMATION}</svg>`;
      case "missing":
      case "error":
        return `<svg viewBox="0 0 22 22"><rect x="1" y="1" width="20" height="20" rx="6" fill="var(--red)"/>${EXCLAMATION}</svg>`;
      case "busy":
        return `<div class="spinner"></div>`;
      default:
        return `<svg viewBox="0 0 22 22"><circle cx="11" cy="11" r="9.5" fill="none" stroke="var(--faint)" stroke-width="1.6" stroke-dasharray="3 3"/></svg>`;
    }
  }

  /** Two circling arrows: "updates automatically". */
  static get auto() {
    return `<svg viewBox="0 0 16 16"><path d="M13 8a5 5 0 0 1-8.6 3.5M3 8a5 5 0 0 1 8.6-3.5M11.8 1.8v2.8H9M4.2 14.2v-2.8H7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  }

  /** Chevron for folding a category (rotated by CSS when folded). */
  static get chevron() {
    return `<svg viewBox="0 0 16 16"><path d="m4.5 6 3.5 3.5L11.5 6" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  }

  /** Download arrow. */
  static get download() {
    return `<svg class="ico" viewBox="0 0 16 16"><path d="M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  }

  /** Close cross. */
  static get close() {
    return `<svg viewBox="0 0 20 20"><path d="m5.5 5.5 9 9m0-9-9 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>`;
  }
}

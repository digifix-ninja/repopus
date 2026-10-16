// Base class for the panels that slide in from the right. Only one is open at a time.

let current = null; // the open panel, if any

export class Sheet {
  /** The dimmed background behind an open panel. */
  static get scrim() {
    return document.querySelector("#scrim");
  }

  /** The open panel, if any. */
  static get current() {
    return current;
  }

  constructor(element) {
    this.el = element;
  }

  /** Whether this panel is showing. */
  get isOpen() {
    return Sheet.current === this;
  }

  /** Slides the panel in (closing any other one). */
  open() {
    if (Sheet.current && Sheet.current !== this) Sheet.current.close();
    current = this;
    this.el.classList.add("on");
    this.el.setAttribute("aria-hidden", "false");
    Sheet.scrim.classList.add("on");
  }

  /** Slides the panel out. */
  close() {
    if (current === this) current = null;
    this.el.classList.remove("on");
    this.el.setAttribute("aria-hidden", "true");
    if (!Sheet.current) Sheet.scrim.classList.remove("on");
    this.onClosed();
  }

  /** Hook for subclasses. */
  onClosed() {}

  /** Closes whichever panel is open. */
  static closeAll() {
    Sheet.current?.close();
  }

  /** Re-renders, keeping the scroll position. Identical HTML is skipped (unless
   *  `force`), so hover states and animations don't restart. Returns whether it redrew. */
  renderKeepingScroll(html, force = false) {
    if (!force && html === this.lastHtml) return false;
    this.lastHtml = html;
    const scroll = this.el.querySelector(".sheet-body")?.scrollTop || 0;
    this.el.innerHTML = html;
    const body = this.el.querySelector(".sheet-body");
    if (body) body.scrollTop = scroll;
    return true;
  }

  // ---------------------------------------------------------------- shared form controls

  /** Two-option switch, e.g. Zip archive / Unpacked folder. */
  static segmented(name, value, options) {
    const buttons = options
      .map(([v, label]) => `<button data-v="${v}" class="${v === value ? "on" : ""}">${label}</button>`)
      .join("");
    return `<div class="seg2" data-seg="${name}">${buttons}</div>`;
  }

  /** On/off toggle. */
  static toggle(name, checked) {
    return `<label class="switch"><input type="checkbox" data-sw="${name}" ${checked ? "checked" : ""}><span></span></label>`;
  }

  /** Close button for the panel header. */
  static closeButton(icon) {
    return `<button class="icon-btn" data-close title="Close (Esc)">${icon}</button>`;
  }

  /** A button that needs a second click within a few seconds to act. */
  static confirm(button, armedText, action) {
    if (button.classList.contains("armed")) return action();
    const original = button.textContent;
    button.classList.add("armed");
    button.textContent = armedText;
    setTimeout(() => {
      button.classList.remove("armed");
      button.textContent = original;
    }, 3500);
  }
}

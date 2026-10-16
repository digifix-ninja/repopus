// Short messages at the bottom of the window.

export class Toast {
  constructor(element) {
    this.el = element;
    this.timer = null;
  }

  /** Shows `text` for a few seconds (longer, and in red, for errors). */
  show(text, error = false) {
    const base = "toast" + (error ? " error" : "");
    this.el.textContent = text;
    this.el.className = base + " on";
    clearTimeout(this.timer);
    this.timer = setTimeout(() => (this.el.className = base), error ? 5200 : 2800);
  }

  /** Shows an error. */
  error(text) {
    this.show(String(text), true);
  }
}

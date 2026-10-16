// Light / dark look of the window contents. "system" lets the operating system decide.

export class Theme {
  /** Applies "system", "light" or "dark" (the CSS reads <html data-theme>). */
  static apply(appearance) {
    const theme = appearance === "light" || appearance === "dark" ? appearance : "system";
    if (document.documentElement.dataset.theme !== theme) document.documentElement.dataset.theme = theme;
  }
}

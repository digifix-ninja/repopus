// Turning data into text for the screen.

export class Format {
  /** Escapes text for safe use inside HTML. */
  static esc(value) {
    const map = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
    return String(value ?? "").replace(/[&<>"']/g, c => map[c]);
  }

  /** "just now", "5 min ago", "3 h ago", "yesterday", or a date. */
  static ago(iso) {
    if (!iso) return "never";
    const seconds = (Date.now() - new Date(iso).getTime()) / 1000;
    if (seconds < 45) return "just now";
    if (seconds < 3600) return `${Math.round(seconds / 60)} min ago`;
    if (seconds < 86400) return `${Math.round(seconds / 3600)} h ago`;
    if (seconds < 172800) return "yesterday";
    if (seconds < 7 * 86400) return `${Math.round(seconds / 86400)} days ago`;
    return new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
  }

  /** Date and time, e.g. "16 Oct 2026, 09:12". */
  static when(iso) {
    if (!iso) return "–";
    return new Date(iso).toLocaleString(undefined, {
      day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit",
    });
  }

  /** "12 MB" style sizes. */
  static bytes(n) {
    if (n == null) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let unit = 0;
    let value = n;
    while (value >= 1000 && unit < units.length - 1) { value /= 1000; unit++; }
    return unit === 0 ? `${n} B` : `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
  }

  /** Text for a busy row: "Downloading · 3.2 MB of 14 MB", "Verifying…", … */
  static busy(busy) {
    if (busy.phase !== "downloading") return busy.text;
    if (!busy.bytes) return "Starting download…";
    const of = busy.total && busy.bytes <= busy.total ? ` of ${this.bytes(busy.total)}` : "";
    return `Downloading · ${this.bytes(busy.bytes)}${of}`;
  }

  /** Share of a download done (0–1), or `null` when the size isn't known. */
  static fraction(busy) {
    if (busy.phase !== "downloading" || !busy.total || busy.bytes > busy.total) return null;
    return Math.min(busy.bytes / busy.total, 0.98); // the last bit fills when it's done
  }

  /** First 7 characters of a commit id. */
  static sha(sha) {
    return sha ? sha.slice(0, 7) : "–";
  }

  /** "1 repository" / "5 repositories". */
  static repositories(n) {
    return `${n} ${n === 1 ? "repository" : "repositories"}`;
  }

  /** A path for display: left-to-right marks keep the leading "/" in place. */
  static path(path) {
    return `&lrm;${this.esc(path)}&lrm;`;
  }
}

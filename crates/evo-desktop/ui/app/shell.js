// Navigation guard for the in-app shell. The scheme handler is the real
// boundary — it cannot serve anything but evo-app://app — and this keeps the
// page from even trying: no remote link, no child window, no form post.
(() => {
  if (!location.protocol.startsWith("evo-app")) return;
  const allowed = (href) => {
    try {
      const url = new URL(href, location.href);
      return url.protocol === "evo-app:" && url.host === "app";
    } catch {
      return false;
    }
  };
  document.addEventListener(
    "click",
    (event) => {
      const link = event.target?.closest?.("a[href]");
      if (!link) return;
      if (!allowed(link.getAttribute("href"))) {
        event.preventDefault();
        event.stopPropagation();
      }
    },
    true
  );
  document.addEventListener(
    "submit",
    (event) => {
      if (!allowed(event.target?.getAttribute?.("action") ?? "")) event.preventDefault();
    },
    true
  );
  window.open = () => null;
})();

window.applyEvoTheme = (choice) => {
  const theme = ["auto", "day", "dusk"].includes(choice) ? choice : "dusk";
  document.documentElement.dataset.theme = theme;
  window.dispatchEvent(new CustomEvent("evo-theme-changed", { detail: theme }));
};

const systemTheme = matchMedia("(prefers-color-scheme: dark)");
systemTheme.addEventListener?.("change", () => {
  if (document.documentElement.dataset.theme === "auto") {
    window.dispatchEvent(new CustomEvent("evo-theme-changed", { detail: "auto" }));
  }
});

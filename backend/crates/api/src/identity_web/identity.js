"use strict";

// Local presentation only. Disabled credential fields stay inert in this stage.
// Never read the URL fragment, tokens, cookies, storage or account data here.
for (const toggle of document.querySelectorAll("[data-toggle]")) {
  toggle.addEventListener("click", () => {
    const input = document.getElementById(toggle.dataset.toggle);
    if (!input || input.disabled) return;
    const visible = input.type === "password";
    input.type = visible ? "text" : "password";
    toggle.textContent = visible ? "Hide" : "Show";
    toggle.setAttribute("aria-pressed", String(visible));
  });
}

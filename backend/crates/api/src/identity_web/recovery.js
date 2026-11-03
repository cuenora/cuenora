"use strict";

(() => {
  const form = document.getElementById("recovery-form");
  const reset = form.dataset.mode === "reset";
  const submit = document.getElementById("submit");
  const status = document.getElementById("status");
  const email = document.getElementById("email");
  const password = document.getElementById("new-password");
  const confirmation = document.getElementById("confirm-password");
  const toggles = document.querySelectorAll("[data-toggle]");
  let token = "";
  let busy = false;
  let closed = false;
  let controller;

  function announce(message) {
    status.textContent = message;
    status.focus();
  }
  function controls(disabled) {
    for (const input of [email, password, confirmation]) {
      if (input) input.disabled = disabled;
    }
    for (const toggle of toggles) toggle.disabled = disabled;
    submit.disabled = disabled;
    form.setAttribute("aria-busy", String(busy));
  }
  function clearPasswords() {
    if (password) { password.value = ""; password.type = "password"; }
    if (confirmation) { confirmation.value = ""; confirmation.type = "password"; }
    for (const toggle of toggles) {
      toggle.textContent = "Show";
      toggle.setAttribute("aria-pressed", "false");
      toggle.setAttribute("aria-label", `Show ${toggle.dataset.toggle === "new-password" ? "new password" : "confirm password"}`);
    }
  }
  function close(message) {
    token = "";
    closed = true;
    clearPasswords();
    if (email) email.value = "";
    controls(true);
    if (message) announce(message);
  }

  let fragment = location.hash;
  const queryPresent = Boolean(location.search);
  try {
    history.replaceState(null, "", location.pathname);
  } catch {
    close("This page cannot securely open the recovery link. Reopen the original email in a supported browser.");
    return;
  }
  if (reset) {
    const match = /^#token=(pr1_[a-f0-9]{64})$/.exec(fragment);
    if (queryPresent || !match) {
      close("This link cannot be used. Request a fresh recovery email and open its original link.");
      return;
    }
    token = match[1];
  }
  fragment = "";
  status.textContent = reset
    ? "Your password changes only when you select Change password."
    : "Enter your email to request recovery instructions.";
  controls(false);

  for (const toggle of toggles) {
    toggle.addEventListener("click", () => {
      const input = document.getElementById(toggle.dataset.toggle);
      const visible = input.type === "password";
      input.type = visible ? "text" : "password";
      toggle.textContent = visible ? "Hide" : "Show";
      toggle.setAttribute("aria-pressed", String(visible));
      toggle.setAttribute("aria-label", `${visible ? "Hide" : "Show"} ${input.id === "new-password" ? "new password" : "confirm password"}`);
    });
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (busy || closed || !form.reportValidity()) return;
    for (const input of [email, password, confirmation]) {
      if (input) input.setAttribute("aria-invalid", "false");
    }
    if (reset) {
      const length = Array.from(password.value).length;
      if (length < 15 || length > 128) {
        password.setAttribute("aria-invalid", "true");
        announce("Use a password with 15–128 characters.");
        return;
      }
      if (password.value !== confirmation.value) {
        confirmation.setAttribute("aria-invalid", "true");
        announce("Passwords do not match. Enter the same password in both fields.");
        return;
      }
    }
    busy = true;
    controls(true);
    status.textContent = reset ? "Changing password…" : "Requesting recovery email…";
    controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 30000);
    try {
      const pending = fetch(reset
        ? "/v1/auth/password-recovery/confirm"
        : "/v1/auth/password-recovery/request", {
        method: "POST",
        credentials: "omit",
        redirect: "error",
        cache: "no-store",
        referrerPolicy: "no-referrer",
        headers: { "Content-Type": "application/json", "Accept": "application/json" },
        body: JSON.stringify(reset
          ? { token, new_password: password.value }
          : { email: email.value }),
        signal: controller.signal,
      });
      clearPasswords();
      const response = await pending;
      if (closed) return;
      if (reset && response.status === 204) {
        close("Password changed. You are signed out on every device. Sign in again with your new password using your usual Cuenora client. Browser sign-in is not available yet. Email verification is unchanged.");
      } else if (!reset && response.status === 202) {
        close("Check your inbox. If an eligible Cuenora account matches that address, we will send recovery instructions. Delivery may take a moment; check your spam folder too.");
      } else if (response.status === 429) {
        announce("Too many attempts. Please wait before trying again.");
      } else if (response.status === 400) {
        const failure = await response.json();
        if (reset && failure.code === "invalid_challenge") {
          close("This link cannot be used. It may have expired or already been used. Request a fresh recovery email.");
        } else if (failure.code === "invalid_request") {
          (reset ? password : email).setAttribute("aria-invalid", "true");
          announce(reset ? "Choose a different password that meets the policy. Common passwords are not accepted."
            : "Enter a valid email address.");
        } else {
          throw new Error("Unexpected recovery response");
        }
      } else {
        throw new Error("Unconfirmed recovery response");
      }
    } catch {
      if (!closed) {
        if (reset) close("We could not confirm the result. Your password may have changed. Try signing in with your new password using your usual Cuenora client. If it does not work, request a fresh recovery email. Do not resubmit this link.");
        else announce("We could not confirm the email request. Delivery may still occur. Check your inbox or try again later.");
      }
    } finally {
      clearTimeout(timeout);
      busy = false;
      controls(closed);
    }
  });
  window.addEventListener("pagehide", () => {
    if (controller) controller.abort();
    close();
  });
  window.addEventListener("pageshow", (event) => {
    if (event.persisted) close("Reopen the original email link to continue, or request a fresh recovery email.");
  });
})();

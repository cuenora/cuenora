"use strict";

(() => {
  const form = document.getElementById("login-form");
  const email = document.getElementById("email");
  const password = document.getElementById("password");
  const submit = document.getElementById("submit");
  const status = document.getElementById("status");
  const verify = document.getElementById("verify-button");
  const request = document.getElementById("request-button");
  const logout = document.getElementById("logout-button");
  const credentials = { access: "", refresh: "" };
  let challenge = "";
  let busy = false;
  let closed = false;
  let controller;

  function announce(message) {
    status.textContent = message;
    status.focus();
  }

  function clearCredentials(keepChallenge = false) {
    credentials.access = "";
    credentials.refresh = "";
    if (!keepChallenge) challenge = "";
    email.value = "";
    password.value = "";
    verify.disabled = true;
    request.disabled = true;
    logout.disabled = true;
    form.setAttribute("aria-busy", "false");
  }

  function close(message) {
    closed = true;
    clearCredentials();
    email.disabled = true;
    password.disabled = true;
    submit.disabled = true;
    if (message) announce(message);
  }

  function setSignedIn() {
    if (closed) return;
    email.disabled = true;
    password.disabled = true;
    submit.disabled = true;
    verify.disabled = !challenge;
    request.disabled = false;
    logout.disabled = false;
    announce(challenge
      ? "You are signed in. Confirm email verification to continue."
      : "You are signed in. You can request a verification email or sign out.");
  }

  function clearPassword() {
    password.value = "";
    password.type = "password";
  }

  async function send(path, body, bearer, parseJson = false) {
    const requestController = new AbortController();
    controller = requestController;
    const timeout = setTimeout(() => requestController.abort(), 30000);
    try {
      const response = await fetch(path, {
        method: "POST",
        credentials: "omit",
        redirect: "error",
        cache: "no-store",
        referrerPolicy: "no-referrer",
        headers: {
          "Content-Type": "application/json",
          "Accept": "application/json",
          ...(bearer ? { "Authorization": `Bearer ${bearer}` } : {}),
        },
        body: JSON.stringify(body),
        signal: requestController.signal,
      });
      const data = parseJson ? await response.json() : undefined;
      return { response, data };
    } finally {
      clearTimeout(timeout);
      if (controller === requestController) controller = undefined;
    }
  }

  function setBusy(value) {
    busy = value;
    form.setAttribute("aria-busy", String(value));
    if (value) {
      email.disabled = true;
      password.disabled = true;
      submit.disabled = true;
      verify.disabled = true;
      request.disabled = true;
      logout.disabled = true;
    } else if (credentials.access && !closed) {
      email.disabled = true;
      password.disabled = true;
      submit.disabled = true;
      verify.disabled = !challenge;
      request.disabled = false;
      logout.disabled = false;
    } else if (!closed) {
      email.disabled = false;
      password.disabled = false;
      submit.disabled = false;
    }
  }

  let fragment = location.hash;
  const queryPresent = Boolean(location.search);
  const fragmentPresent = Boolean(fragment);
  let match = /^#token=(ev1_[a-f0-9]{64})$/.exec(fragment);
  try {
    history.replaceState(null, "", location.pathname);
  } catch {
    close("This page cannot securely open the sign-in link. Reopen the original email in a supported browser.");
    return;
  }
  fragment = "";
  if (queryPresent || (fragmentPresent && !match)) {
    close("This link cannot be used. Reopen the original email link.");
    return;
  }
  if (match) {
    challenge = match[1];
    match = null;
  }
  status.textContent = challenge
    ? "Sign in to the account that received this email. Verification requires your confirmation."
    : "Sign in to your Cuenora account. You can request email verification after signing in.";
  email.disabled = false;
  password.disabled = false;
  submit.disabled = false;

  form.addEventListener("submit", async event => {
    event.preventDefault();
    if (busy || closed || !form.reportValidity()) return;
    setBusy(true);
    status.textContent = "Signing in…";
    let pending;
    try {
      pending = send("/v1/auth/login", { email: email.value, password: password.value }, undefined, true);
      clearPassword();
      const { response, data: pair } = await pending;
      if (closed) return;
      if (response.status === 200) {
        if (!pair || typeof pair.access_token !== "string" || !pair.access_token ||
            typeof pair.refresh_token !== "string" || !pair.refresh_token ||
            pair.token_type !== "Bearer" || !Number.isSafeInteger(pair.expires_in) || pair.expires_in <= 0 ||
            !Number.isSafeInteger(pair.refresh_expires_in) || pair.refresh_expires_in <= 0) {
          throw new Error("Unexpected sign-in response");
        }
        credentials.access = pair.access_token;
        credentials.refresh = pair.refresh_token;
        setSignedIn();
      } else if (response.status === 429) {
        announce("Too many sign-in attempts. Please wait before trying again.");
      } else if (response.status === 401 || response.status === 400) {
        announce("Sign-in failed. Check your email and password, then try again.");
      } else {
        throw new Error("Unconfirmed sign-in response");
      }
    } catch {
      if (!closed) {
        clearCredentials(true);
        announce("We could not confirm sign-in. Try again; no session is available in this page.");
      }
    } finally {
      if (!closed) setBusy(false);
    }
  });

  verify.addEventListener("click", async () => {
    if (busy || closed || !credentials.access || !challenge) return;
    setBusy(true);
    status.textContent = "Confirming email verification…";
    try {
      const { response } = await send("/v1/auth/email-verification/confirm", { token: challenge }, credentials.access);
      if (closed) return;
      if (response.status === 204) {
        challenge = "";
        verify.disabled = true;
        announce("Your email is verified.");
      } else if (response.status === 400) {
        close("This verification link cannot be used. It may have expired or already been used. Request a fresh verification email.");
      } else if (response.status === 429) {
        announce("Too many attempts. Please wait before trying again.");
      } else {
        throw new Error("Unconfirmed verification result");
      }
    } catch {
      if (!closed) close("We could not confirm verification. Sign in again and check your account before retrying.");
    } finally {
      if (!closed) setBusy(false);
    }
  });

  request.addEventListener("click", async () => {
    if (busy || closed || !credentials.access) return;
    setBusy(true);
    status.textContent = "Requesting verification email…";
    try {
      const { response } = await send("/v1/auth/email-verification/request", {}, credentials.access);
      if (closed) return;
      if (response.status === 202) {
        announce("If your account needs verification, check your inbox.");
      } else if (response.status === 429) {
        announce("Too many attempts. Please wait before trying again.");
      } else {
        throw new Error("Unconfirmed email request result");
      }
    } catch {
      if (!closed) close("We could not confirm the email request. It may still be processing; check your inbox before requesting again.");
    } finally {
      if (!closed) setBusy(false);
    }
  });

  logout.addEventListener("click", async () => {
    if (busy || closed || !credentials.refresh) return;
    setBusy(true);
    status.textContent = "Signing out…";
    let logoutConfirmed = false;
    try {
      const { response } = await send("/v1/auth/logout", { refresh_token: credentials.refresh });
      logoutConfirmed = response.status === 204;
    } catch {
      // A failed logout must still remove this page's local authority.
    } finally {
      if (!closed) {
        clearCredentials();
        closed = false;
        setBusy(false);
        email.disabled = false;
        password.disabled = false;
        submit.disabled = false;
        announce(logoutConfirmed
          ? "You are signed out. Sign in again to continue."
          : "This page is signed out, but server sign-out could not be confirmed. Your session may remain active.");
      }
    }
  });

  window.addEventListener("pagehide", () => {
    if (controller) controller.abort();
    close();
  });
  window.addEventListener("pageshow", event => {
    if (event.persisted) close("This sign-in page was closed. Reload it to sign in again.");
  });
})();

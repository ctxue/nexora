import { useCallback, useEffect, useRef, useState } from "react";

import {
  platform,
  type GithubDeviceStart,
  type GithubStatus,
} from "../../platform/tauri";

const EMPTY_STATUS: GithubStatus = {
  configured: false,
  connected: false,
  login: null,
  avatarUrl: null,
  hasGist: false,
};

function loginError(code: string): string {
  if (code.startsWith("githubNetworkError")) return "githubNetworkFailed";
  if (code.startsWith("githubResponseError")) return "githubResponseFailed";
  if (code === "access_denied") return "githubAccessDenied";
  if (code === "expired_token" || code === "token_expired") {
    return "githubCodeExpired";
  }
  if (code === "githubClientIdMissing") return "githubClientIdMissing";
  if (code === "githubBrowserOpenFailed") return "githubBrowserOpenFailed";
  return "githubLoginFailed";
}

export function useGithubAuth() {
  const [status, setStatus] = useState<GithubStatus>(EMPTY_STATUS);
  const [device, setDevice] = useState<GithubDeviceStart | null>(null);
  const [busy, setBusy] = useState(false);
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const deviceRef = useRef<GithubDeviceStart | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await platform.githubStatus());
    } catch {
      setStatus(EMPTY_STATUS);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  const startLogin = useCallback(async () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const next = await platform.githubDeviceStart();
      deviceRef.current = next;
      setDevice(next);
      if (!next.browserOpened) {
        setError("githubBrowserOpenFailed");
      }
    } catch (reason) {
      setError(loginError(reason instanceof Error ? reason.message : String(reason)));
    } finally {
      setBusy(false);
    }
  }, [busy]);

  const cancelLogin = useCallback(() => {
    deviceRef.current = null;
    setDevice(null);
    setError(null);
  }, []);

  const openVerificationPage = useCallback(async () => {
    if (!device || opening) return;
    setOpening(true);
    setError(null);
    try {
      await platform.openUrl(device.verificationUri, "default", false);
    } catch {
      setError("githubBrowserOpenFailed");
    } finally {
      setOpening(false);
    }
  }, [device, opening]);

  useEffect(() => {
    const active = deviceRef.current;
    if (!device) return;

    let cancelled = false;
    let timer: number | undefined;
    let delay = Math.max(5, device.interval) * 1000;
    const expiresAt = Date.now() + device.expiresIn * 1000;

    const poll = async () => {
      if (cancelled || deviceRef.current !== active) return;
      if (Date.now() >= expiresAt) {
        setError("githubCodeExpired");
        cancelLogin();
        return;
      }

      try {
        const result = await platform.githubDevicePoll(device.deviceCode);
        if (cancelled || deviceRef.current !== active) return;
        if (result.status === "authorized") {
          deviceRef.current = null;
          setDevice(null);
          setError(null);
          await refreshStatus();
          await platform.openSettingsWindow().catch(() => {});
          return;
        }
        if (result.status === "slow_down") delay += 5000;
      } catch (reason) {
        if (cancelled) return;
        setError(loginError(reason instanceof Error ? reason.message : String(reason)));
        cancelLogin();
        return;
      }

      timer = window.setTimeout(() => void poll(), delay);
    };

    timer = window.setTimeout(() => void poll(), delay);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [cancelLogin, device, refreshStatus]);

  const logout = useCallback(async () => {
    setBusy(true);
    try {
      await platform.githubLogout();
      await refreshStatus();
    } finally {
      setBusy(false);
    }
  }, [refreshStatus]);

  return {
    status,
    device,
    busy,
    opening,
    error,
    startLogin,
    cancelLogin,
    openVerificationPage,
    logout,
    refreshStatus,
  };
}

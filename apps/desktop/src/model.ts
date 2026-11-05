export type Locale = "ru" | "en";
export type Page = "home" | "devices" | "installer" | "logs" | "settings";
export interface DeviceInfo {
  ProductType: string;
  ProductVersion: string;
  BuildVersion: string;
  CPUArchitecture: string;
}
export interface DeviceView {
  device: {
    id: string;
    info: DeviceInfo | null;
    trust: string;
    failure: string | null;
  };
  compatibility: {
    status: "research_target" | "unverified";
    jailbreak_available: false;
    reason: string;
  } | null;
}
export interface Scan {
  devices: DeviceView[];
  failure: string | null;
  installer: { Ok?: "legacy" | "modern"; Err?: unknown };
}
export interface Ipa {
  bundle_id: string;
  version: string;
  minimum_ios: string;
  signing: string;
}
export interface Entry {
  timestamp: number;
  level: "info" | "warning" | "error";
  event: string;
}
export interface Preferences {
  locale: Locale;
  theme: "dark" | "light";
  mask: boolean;
  scanOnLaunch: boolean;
}

export function escapeHtml(value: unknown): string {
  return String(value).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
}
export function maskedId(id: string, mask: boolean): string {
  return mask ? `${id.slice(0, 6)}••••${id.slice(-4)}` : id;
}
export function canInstall(
  scan: Scan | null,
  selected: DeviceView | undefined,
  ipa: Ipa | null,
  busy: boolean,
): boolean {
  return (
    !busy &&
    !!ipa &&
    !!scan?.installer.Ok &&
    !scan.failure &&
    !!selected?.device.info &&
    selected.device.trust === "lockdown_readable" &&
    !selected.device.failure
  );
}
export function preferences(raw: string | null): Preferences {
  const fallback: Preferences = {
    locale: "ru",
    theme: "dark",
    mask: true,
    scanOnLaunch: false,
  };
  try {
    const value = JSON.parse(raw ?? "{}") as Partial<Preferences> | null;
    if (!value || typeof value !== "object") return fallback;
    return {
      locale: value.locale === "en" ? "en" : "ru",
      theme: value.theme === "light" ? "light" : "dark",
      mask: typeof value.mask === "boolean" ? value.mask : true,
      scanOnLaunch: value.scanOnLaunch === true,
    };
  } catch {
    return fallback;
  }
}

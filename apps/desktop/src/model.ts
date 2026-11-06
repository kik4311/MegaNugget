export type Locale = "ru" | "en";
export type Page =
  "home" | "devices" | "installer" | "tweaks" | "logs" | "settings";
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

export type SigningPhase =
  | "idle"
  | "authenticating"
  | "two_factor"
  | "select_team"
  | "provisioning"
  | "signing"
  | "signed"
  | "failed"
  | "cancelled";
export interface SigningView {
  status: {
    phase: SigningPhase;
    teams: { id: string; name: string }[];
    error: string | null;
  };
  ipa: Ipa | null;
}
export function signingActive(phase: SigningPhase): boolean {
  return !["idle", "signed", "failed", "cancelled"].includes(phase);
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
    ipa?.signing === "artifacts_present_not_verified" &&
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

/// Offline tweak drafts: types mirror `crates/customization` serialization.
export type TweakStatus =
  | "draft_candidate"
  | "requires_newer_ios"
  | "upstream_unavailable"
  | "hardware_mismatch"
  | "outside_legacy_scope"
  | "invalid_device"
  | "mechanism_not_ported";
export type TweakKind = "toggle" | "text" | "number";
export type TweakValue =
  | { type: "bool"; value: boolean }
  | { type: "int"; value: number }
  | { type: "double"; value: number }
  | { type: "string"; value: string };
export interface TweakSummary {
  id: string;
  title: string;
  section: string;
  detail: string | null;
  warning: string | null;
  minimum_ios: string | null;
  maximum_ios: string | null;
  status: TweakStatus;
  default_value: TweakValue;
  destination: string;
  key: string;
  kind: TweakKind;
  writes_whole_dict: boolean;
}
export interface TweakCounts {
  total: number;
  candidates: number;
  requires_newer_ios: number;
  upstream_unavailable: number;
  hardware_mismatch: number;
  outside_legacy_scope: number;
  invalid_device: number;
  mechanism_not_ported: number;
}
export interface TweaksCatalogView {
  repository: string;
  revision: string;
  registry_count: number;
  counts: TweakCounts;
  entries: TweakSummary[];
}
export interface DraftPreview {
  files: { domain: string; relative_path: string; xml: string }[];
  selected_count: number;
  applied_to_device: boolean;
}
export interface TweakTarget {
  model: string;
  system_version: string;
  architecture: string;
}

/// Localized status wording for the Tweaks page; the backend only ships codes.
export const tweakStatusHints: Record<TweakStatus, [string, string]> = {
  draft_candidate: [
    "Кандидат для черновика · работа на устройстве не проверена",
    "Draft candidate · on-device behavior is unverified",
  ],
  requires_newer_ios: [
    "Upstream требует более новую iOS",
    "Upstream requires a newer iOS",
  ],
  upstream_unavailable: [
    "Отключено или ограничено в upstream",
    "Disabled or restricted upstream",
  ],
  hardware_mismatch: [
    "Ограничение модели или версии устройства",
    "Device model/version limitation",
  ],
  outside_legacy_scope: [
    "Вне диапазона iOS 15–17",
    "Outside the iOS 15–17 range",
  ],
  invalid_device: [
    "Недостаточно корректных данных устройства",
    "Device data is not valid enough",
  ],
  mechanism_not_ported: [
    "Специальный механизм не перенесён",
    "Specialized mechanism not ported",
  ],
};

/// Model tables ported verbatim from the Swift adapter (`deviceStatus`):
/// they are existence bounds, not evidence that any tweak works.
export const tweakTargetModelGroups: {
  group: [string, string];
  models: string[];
}[] = [
  {
    group: [
      "iOS 15 · первые 64-битные (iPhone 6s…iPhone SE)",
      "iOS 15 · first ARM64 (iPhone 6s…iPhone SE)",
    ],
    models: [
      "iPhone8,1",
      "iPhone8,2",
      "iPhone8,4",
      "iPhone9,1",
      "iPhone9,2",
      "iPhone9,3",
      "iPhone9,4",
    ],
  },
  {
    group: [
      "до iOS 16 включительно (iPhone 7…SE 2)",
      "through iOS 16 (iPhone 7…SE 2)",
    ],
    models: [
      "iPhone10,1",
      "iPhone10,2",
      "iPhone10,3",
      "iPhone10,4",
      "iPhone10,5",
      "iPhone10,6",
    ],
  },
  {
    group: ["iOS 15–17 (Face ID)", "iOS 15–17 (Face ID)"],
    models: [
      "iPhone11,2",
      "iPhone11,4",
      "iPhone11,6",
      "iPhone11,8",
      "iPhone12,1",
      "iPhone12,3",
      "iPhone12,5",
      "iPhone12,8",
      "iPhone13,1",
      "iPhone13,2",
      "iPhone13,3",
      "iPhone13,4",
      "iPhone14,2",
      "iPhone14,3",
      "iPhone14,4",
      "iPhone14,5",
      "iPhone14,6",
    ],
  },
  {
    group: [
      "iOS 16–17 (iPhone 13 mini…14 Plus)",
      "iOS 16–17 (iPhone 13 mini…14 Plus)",
    ],
    models: ["iPhone14,7", "iPhone14,8", "iPhone15,2", "iPhone15,3"],
  },
  {
    group: ["iOS 17 (Dynamic Island)", "iOS 17 (Dynamic Island)"],
    models: ["iPhone15,4", "iPhone15,5", "iPhone16,1", "iPhone16,2"],
  },
];

/// Section labels: upstream English titles are kept verbatim in both locales,
/// because they name the upstream registry, not our own terms.
export const tweakSectionLabels: Record<string, [string, string]> = {
  liquidGlass: ["Liquid Glass", "Liquid Glass"],
  springboard: ["SpringBoard", "SpringBoard"],
  internalOptions: ["Internal Options", "Internal Options"],
  daemons: ["Daemons to Disable", "Daemons to Disable"],
};

export function tweakMatchesFilter(
  entry: TweakSummary,
  search: string,
  onlyCandidates: boolean,
): boolean {
  if (onlyCandidates && entry.status !== "draft_candidate") return false;
  const needle = search.trim().toLowerCase();
  if (!needle) return true;
  return `${entry.id} ${entry.title} ${entry.section}`
    .toLowerCase()
    .includes(needle);
}

/// Parse the editor's text field into a plist scalar the backend will accept
/// for this entry. `null` means "not a valid value for this kind" — never a
/// coerced guess.
export function parseTweakValue(
  entry: TweakSummary,
  raw: string,
): TweakValue | null {
  const text = raw.trim();
  if (!text) return null;
  if (entry.kind === "toggle") {
    if (text === "true") return { type: "bool", value: true };
    if (text === "false") return { type: "bool", value: false };
    return null;
  }
  if (entry.kind === "text") {
    if (entry.writes_whole_dict) return null;
    return { type: "string", value: raw };
  }
  // Number: the registry default decides int vs double, like upstream.
  if (entry.default_value.type === "int") {
    if (!/^-?\d+$/.test(text)) return null;
    const parsed = Number(text);
    if (!Number.isSafeInteger(parsed)) return null;
    return { type: "int", value: parsed };
  }
  if (entry.default_value.type === "double") {
    const parsed = Number(text);
    if (!Number.isFinite(parsed)) return null;
    return { type: "double", value: parsed };
  }
  return null;
}

export function tweakValueDisplay(value: TweakValue): string {
  return String(value.value);
}

export function tweakValueEquals(a: TweakValue, b: TweakValue): boolean {
  return a.type === b.type && a.value === b.value;
}

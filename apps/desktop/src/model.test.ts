import { describe, expect, it } from "vitest";
import {
  canInstall,
  escapeHtml,
  maskedId,
  parseTweakValue,
  preferences,
  signingActive,
  tweakMatchesFilter,
  tweakSectionLabels,
  tweakStatusHints,
  tweakTargetModelGroups,
  tweakValueDisplay,
  tweakValueEquals,
  type DeviceView,
  type Scan,
  type TweakSummary,
} from "./model";

function summary(overrides: Partial<TweakSummary> = {}): TweakSummary {
  return {
    id: "SBHideLowPowerAlerts",
    title: "Hide Low Power Alerts",
    section: "springboard",
    detail: null,
    warning: null,
    minimum_ios: null,
    maximum_ios: null,
    status: "draft_candidate",
    default_value: { type: "bool", value: true },
    destination: "/var/Managed Preferences/mobile/com.apple.springboard.plist",
    key: "SBHideLowPowerAlerts",
    kind: "toggle",
    writes_whole_dict: false,
    ...overrides,
  };
}
describe("safe presentation", () => {
  it("escapes device/archive content", () =>
    expect(escapeHtml('<img onerror="x">&')).toBe(
      "&lt;img onerror=&quot;x&quot;&gt;&amp;",
    ));
  it("masks identifiers by default", () =>
    expect(maskedId("0123456789abcdef", true)).toBe("012345••••cdef"));
  it("validates persisted settings", () => {
    for (const raw of ["broken", "null", '{"theme":"bad","mask":"false"}']) {
      expect(preferences(raw).theme).toBe("dark");
      expect(preferences(raw).mask).toBe(true);
    }
    expect(preferences(null).scanOnLaunch).toBe(false);
  });
});
describe("install gating", () => {
  const device: DeviceView = {
    device: {
      id: "test",
      trust: "lockdown_readable",
      failure: null,
      info: {
        ProductType: "iPhone9,1",
        ProductVersion: "15.8.8",
        BuildVersion: "test",
        CPUArchitecture: "arm64",
      },
    },
    compatibility: null,
  };
  const scan: Scan = {
    devices: [device],
    failure: null,
    installer: { Ok: "legacy" },
  };
  const ipa = {
    bundle_id: "org.meganugget.ios",
    version: "1",
    minimum_ios: "15.0",
    signing: "artifacts_present_not_verified",
  };
  it("requires every prerequisite", () => {
    expect(canInstall(scan, device, ipa, false)).toBe(true);
    expect(canInstall(scan, device, ipa, true)).toBe(false);
    expect(
      canInstall(scan, device, { ...ipa, signing: "requires_signing" }, false),
    ).toBe(false);
    expect(canInstall(scan, device, null, false)).toBe(false);
    expect(canInstall(null, device, ipa, false)).toBe(false);
    expect(
      canInstall(
        { ...scan, installer: { Err: "missing" } },
        device,
        ipa,
        false,
      ),
    ).toBe(false);
    expect(
      canInstall(
        scan,
        { ...device, device: { ...device.device, trust: "unknown" } },
        ipa,
        false,
      ),
    ).toBe(false);
  });
});

describe("signing state", () => {
  it("keeps pending challenges active and terminal states inactive", () => {
    for (const phase of [
      "authenticating",
      "two_factor",
      "select_team",
      "provisioning",
      "signing",
    ] as const)
      expect(signingActive(phase)).toBe(true);
    for (const phase of ["idle", "signed", "failed", "cancelled"] as const)
      expect(signingActive(phase)).toBe(false);
  });
  it("never persists credentials in preferences", () => {
    const value = preferences(
      '{"email":"private@example.test","password":"secret","code":"123456"}',
    );
    expect(Object.keys(value).sort()).toEqual([
      "locale",
      "mask",
      "scanOnLaunch",
      "theme",
    ]);
  });
});

describe("tweak drafts", () => {
  it("filters by status and search text", () => {
    expect(tweakMatchesFilter(summary(), "", false)).toBe(true);
    expect(tweakMatchesFilter(summary(), "  ", true)).toBe(true);
    expect(
      tweakMatchesFilter(summary({ status: "hardware_mismatch" }), "", true),
    ).toBe(false);
    expect(
      tweakMatchesFilter(summary({ status: "hardware_mismatch" }), "", false),
    ).toBe(true);
    expect(tweakMatchesFilter(summary(), "LowPoWeR", false)).toBe(true);
    expect(tweakMatchesFilter(summary(), "springboard", false)).toBe(true);
    expect(tweakMatchesFilter(summary(), "not-a-tweak", false)).toBe(false);
  });
  it("parses editor text exactly like the compiler validates it", () => {
    const toggle = summary();
    expect(parseTweakValue(toggle, " true ")).toEqual({
      type: "bool",
      value: true,
    });
    expect(parseTweakValue(toggle, "yes")).toBeNull();
    const integer = summary({
      kind: "number",
      default_value: { type: "int", value: 1 },
    });
    expect(parseTweakValue(integer, "-12")).toEqual({
      type: "int",
      value: -12,
    });
    expect(parseTweakValue(integer, "1.5")).toBeNull();
    expect(parseTweakValue(integer, "1e6")).toBeNull();
    expect(parseTweakValue(integer, " 7 ")).toEqual({ type: "int", value: 7 });
    expect(
      parseTweakValue(integer, "9007199254740993"),
      "out of the safe integer range",
    ).toBeNull();
    const double = summary({
      kind: "number",
      default_value: { type: "double", value: 1.0 },
    });
    expect(parseTweakValue(double, "1")).toEqual({
      type: "double",
      value: 1,
    });
    expect(parseTweakValue(double, "-0.5")).toEqual({
      type: "double",
      value: -0.5,
    });
    expect(parseTweakValue(double, "abc")).toBeNull();
    const text = summary({
      kind: "text",
      default_value: { type: "string", value: "" },
    });
    expect(parseTweakValue(text, " value ")).toEqual({
      type: "string",
      value: " value ",
    });
    expect(parseTweakValue(text, "")).toBeNull();
    expect(
      parseTweakValue(
        summary({ kind: "text", writes_whole_dict: true }),
        "anything",
      ),
    ).toBeNull();
  });
  it("renders values without changing their type", () => {
    expect(tweakValueDisplay({ type: "bool", value: false })).toBe("false");
    expect(tweakValueDisplay({ type: "int", value: -7 })).toBe("-7");
    expect(tweakValueDisplay({ type: "double", value: 1.5 })).toBe("1.5");
    expect(tweakValueDisplay({ type: "string", value: "a b" })).toBe("a b");
    expect(
      tweakValueEquals({ type: "int", value: 1 }, { type: "int", value: 1 }),
    ).toBe(true);
    expect(
      tweakValueEquals({ type: "double", value: 1 }, { type: "int", value: 1 }),
      "the plist type is part of the identity",
    ).toBe(false);
    expect(
      tweakValueEquals({ type: "int", value: 1 }, { type: "int", value: 2 }),
    ).toBe(false);
  });
  it("keeps the localized hints and target tables closed", () => {
    const statuses = Object.keys(tweakStatusHints);
    expect(statuses.length).toBe(7);
    for (const [ru, en] of Object.values(tweakStatusHints)) {
      expect(ru.length).toBeGreaterThan(0);
      expect(en.length).toBeGreaterThan(0);
    }
    const groups = tweakTargetModelGroups;
    expect(groups.map((group) => group.group.length)).toEqual(
      groups.map(() => 2),
    );
    expect(groups.flatMap((group) => group.models)).toContain("iPhone9,1");
    expect(groups.flatMap((group) => group.models)).toContain("iPhone9,3");
    expect(tweakSectionLabels.springboard[0]).toBe("SpringBoard");
    expect(tweakSectionLabels.daemons[1]).toBe("Daemons to Disable");
  });
});

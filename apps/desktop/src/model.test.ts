import { describe, expect, it } from "vitest";
import {
  canInstall,
  escapeHtml,
  maskedId,
  preferences,
  signingActive,
  type DeviceView,
  type Scan,
} from "./model";
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

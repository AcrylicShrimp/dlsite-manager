import { beforeEach, describe, expect, it, vi } from "vitest";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
beforeEach(() => { vi.resetModules(); invoke.mockReset(); });

describe("diagnostic failure reporting", () => {
  it("retains only safe summaries while IPC is down, then reconciles once", async () => {
    invoke.mockRejectedValue(new Error("PRIVATE-AUTH-SECRET"));
    const api = await import("./diagnostics");
    await expect(api.diagnosticInvoke("save_account", { password: "PRIVATE-AUTH-SECRET" })).rejects.toThrow();
    const summary = await api.supportSummary();
    expect(summary).not.toContain("PRIVATE-AUTH-SECRET");
    expect(JSON.parse(summary).frontend.pending).toHaveLength(1);
    const payload = invoke.mock.calls.find(([command]) => command === "report_frontend_failure")![1];
    expect(Object.keys(payload.failure).sort()).toEqual(["category", "clientId", "command", "repeatCount"]);
    invoke.mockResolvedValue({});
    await api.diagnosticInvoke("get_settings");
    await vi.waitFor(async () => expect(JSON.parse(await api.supportSummary()).frontend.pending).toHaveLength(0));
  });
  it("does not recursively report reporter failures or duplicate backend-owned errors", async () => {
    invoke.mockRejectedValue("reporter unavailable");
    const api = await import("./diagnostics");
    api.noteFailure("save_account", "failed [diagnostic:run-12345678-1234-1234-1234-123456789abc:op-12345678-1234-1234-1234-123456789abc]");
    expect(invoke).not.toHaveBeenCalled();
    api.noteFailure("native.updater", new Error("SECRET"));
    await Promise.resolve(); await Promise.resolve();
    expect(invoke).toHaveBeenCalledTimes(1);
  });
  it("bounds floods and never stores arbitrary frontend command names", async () => {
    invoke.mockRejectedValue("offline");
    const api = await import("./diagnostics");
    for (let i = 0; i < 1000; i++) api.noteFailure("PRIVATE-COMMAND", "PRIVATE-ERROR");
    const summary = await api.supportSummary();
    expect(summary).not.toContain("PRIVATE");
    expect(JSON.parse(summary).frontend.dropped).toBeGreaterThan(900);
    expect(JSON.parse(summary).frontend.pending).toHaveLength(1);
  });
});

import { describe, expect, it } from "vitest";
import { splitDiagnosticMessage } from "./diagnostic-message";

const runId = "run-96148f68-bb9f-45bd-90cb-c03144f5b5e7";
const operationId = "op-73a57025-8359-41f4-85a6-efad05b6eadd";
const marker = `[diagnostic:${runId}:${operationId}]`;

describe("diagnostic message presentation", () => {
  it("preserves the full correlation IDs separately from the visible error", () => {
    expect(splitDiagnosticMessage(`Update failed: first line\nsecond line ${marker}`)).toEqual({
      message: "Update failed: first line\nsecond line",
      diagnostic: { runId, operationId },
    });
  });
  it("does not hide ordinary, malformed, or non-terminal message text", () => {
    for (const message of ["Library folder is required", "Failed [diagnostic:unknown]", `${marker} extra details`]) {
      expect(splitDiagnosticMessage(message)).toEqual({ message, diagnostic: null });
    }
  });
});

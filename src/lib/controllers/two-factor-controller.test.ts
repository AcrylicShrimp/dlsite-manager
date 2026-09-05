import { expect, it, vi } from "vitest";
import { TwoFactorController } from "./two-factor-controller.svelte";
import type { TwoFactorRequest } from "$lib/model/types";

const request = (id: string): TwoFactorRequest => ({ requestId: id, jobId: id, accountId: id, accountLabel: id, attempt: 1, previousCodeRejected: false });

it("late submission completion cannot clear a newer prompt's submitting state", async () => {
  let finishA!: () => void, finishB!: () => void;
  const port = {
    submitTwoFactorCode: vi.fn().mockImplementationOnce(() => new Promise<void>(resolve => { finishA = resolve; }))
      .mockImplementationOnce(() => new Promise<void>(resolve => { finishB = resolve; })),
    cancelTwoFactor: vi.fn().mockResolvedValue(undefined),
  };
  const controller = new TwoFactorController(vi.fn(), port);
  controller.enqueue(request("a"));
  controller.enqueue(request("b"));
  const a = controller.submit("123456");
  controller.close("a");
  const b = controller.submit("654321");
  finishA(); await a;
  expect(controller.active?.requestId).toBe("b");
  expect(controller.submitting).toBe(true);
  finishB(); await b;
  expect(controller.active).toBeNull();
});

it("retry replacement and disposal reject late state/errors", async () => {
  let reject!: (error: unknown) => void;
  const error = vi.fn();
  const controller = new TwoFactorController(error, {
    submitTwoFactorCode: vi.fn(() => new Promise<void>((_, no) => { reject = no; })),
    cancelTwoFactor: vi.fn().mockResolvedValue(undefined),
  });
  controller.enqueue(request("a"));
  const pending = controller.submit("123456");
  controller.enqueue({ ...request("retry"), jobId: "a", attempt: 2 });
  controller.close("a");
  expect(controller.active?.requestId).toBe("retry");
  expect(controller.submitting).toBe(false);
  controller.dispose();
  controller.enqueue(request("late"));
  reject(new Error("late command failure")); await pending;
  expect(error).not.toHaveBeenCalled();
  expect(controller.queue).toEqual([]);
});

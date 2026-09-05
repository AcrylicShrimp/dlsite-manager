import * as commands from "$lib/api/tauri";
import type { TwoFactorRequest } from "$lib/model/types";

export class TwoFactorController {
  queue = $state<TwoFactorRequest[]>([]);
  private submittingIds = $state<string[]>([]);
  private disposed = false;
  constructor(private onError: (error: unknown) => void,
    private port: Pick<typeof commands, "submitTwoFactorCode" | "cancelTwoFactor"> = commands) {}
  get active() { return this.queue[0] ?? null; }
  get submitting() { return !!this.active && this.submittingIds.includes(this.active.requestId); }

  enqueue(request: TwoFactorRequest) {
    if (this.disposed) return;
    this.queue = [...this.queue.filter(item => item.jobId !== request.jobId), request];
  }
  close(requestId: string) {
    this.queue = this.queue.filter(item => item.requestId !== requestId);
    this.submittingIds = this.submittingIds.filter(id => id !== requestId);
  }
  dispose() { this.disposed = true; this.queue = []; this.submittingIds = []; }

  async submit(code: string) {
    const request = this.active;
    if (!request || this.submitting) return;
    this.submittingIds = [...this.submittingIds, request.requestId];
    try {
      await this.port.submitTwoFactorCode(request.requestId, code);
      this.close(request.requestId);
    } catch (error) {
      if (!this.disposed && this.active?.requestId === request.requestId) this.onError(error);
    } finally {
      this.submittingIds = this.submittingIds.filter(id => id !== request.requestId);
    }
  }
  async cancel() {
    const request = this.active;
    if (!request) return;
    this.close(request.requestId);
    try { await this.port.cancelTwoFactor(request.requestId); }
    catch (error) { if (!this.disposed) this.onError(error); }
  }
}

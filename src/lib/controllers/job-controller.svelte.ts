import * as native from "$lib/api/native";
import * as commands from "$lib/api/tauri";
import type { JobEvent, JobSnapshot } from "$lib/model/types";
import { upsertJob } from "$lib/utils/jobs";

export class JobController {
  jobs = $state<JobSnapshot[]>([]);
  loading = $state(true);
  messages = $state<Record<string, string>>({});

  async load() {
    this.loading = true;

    try {
      this.jobs = await commands.listJobs();
    } finally {
      this.loading = false;
    }
  }

  setMessage(jobId: string, message: string) {
    this.messages = {
      ...this.messages,
      [jobId]: message,
    };
  }

  applyEvent(event: JobEvent) {
    this.jobs = upsertJob(this.jobs, event.snapshot);

    if (event.message) {
      this.setMessage(event.jobId, event.message);
    }
  }

  listen(handler: (event: JobEvent) => void | Promise<void>) {
    return native.listenToJobEvents((event) => {
      this.applyEvent(event);
      void handler(event);
    });
  }
}

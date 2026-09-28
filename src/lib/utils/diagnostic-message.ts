export type DiagnosticReference = { runId: string; operationId: string };
export type DiagnosticTarget = DiagnosticReference & { message: string };

// Only consume the backend's terminal correlation marker. Keep other text intact.
export function splitDiagnosticMessage(message: string): {
  message: string;
  diagnostic: DiagnosticReference | null;
} {
  const match = / \[diagnostic:(run-[0-9a-f-]{36}):(op-[0-9a-f-]{36})\]$/.exec(message);
  if (!match) return { message, diagnostic: null };
  return {
    message: message.slice(0, match.index),
    diagnostic: { runId: match[1], operationId: match[2] },
  };
}

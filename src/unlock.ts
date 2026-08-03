import type { UnlockError, UnlockOutcome } from "./types";

export type Row = {
  id: string;
  path: string;
  name: string;
  status: "ready" | "working" | "done" | "skipped" | "error";
  detail?: string;
  outputPath?: string;
};

export type InvokeFn = (cmd: string, args: Record<string, unknown>) => Promise<unknown>;

const nonEmpty = (m: string | undefined, fallback: string) =>
  m && m.trim() ? m.trim() : fallback;

export function errorMessage(e: unknown): string {
  // invoke() can reject with a plain string (IPC/serialization/permission
  // failures) or an Error, not just a structured UnlockError — handle all three.
  if (typeof e === "string") return nonEmpty(e, "Unexpected error");
  if (e instanceof Error) return nonEmpty(e.message, "Unexpected error");
  const err = e as Partial<UnlockError> | null;
  switch (err?.kind) {
    case "WrongPassword": return "Incorrect password";
    case "Corrupt": return "Could not read PDF";
    case "Io": return nonEmpty(err.message, "File error");
    case "Engine": return nonEmpty(err.message, "Unexpected error");
    default: return nonEmpty(err?.message, "Unexpected error");
  }
}

export async function unlockOne(
  path: string,
  password: string,
  invoke: InvokeFn,
): Promise<Partial<Row>> {
  try {
    const res = (await invoke("unlock_pdf", { inputPath: path, password })) as UnlockOutcome;
    if (res.status === "Unlocked") {
      return { status: "done", outputPath: res.output_path, detail: "Saved" };
    }
    return { status: "skipped", detail: "Not password-protected — skipped" };
  } catch (raw) {
    return { status: "error", detail: errorMessage(raw) };
  }
}

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

export function errorMessage(e: UnlockError): string {
  switch (e.kind) {
    case "WrongPassword": return "Incorrect password";
    case "Corrupt": return "Could not read PDF";
    case "Io": return e.message ?? "File error";
    case "Engine": return e.message ?? "Unexpected error";
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
    return { status: "error", detail: errorMessage(raw as UnlockError) };
  }
}

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

export function nameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

/**
 * Merge dropped/browsed paths into the existing queue. `.pdf` paths (case-
 * insensitive) are added; **re-dropping an existing path resets its row to
 * `ready`** so a file can be retried after a wrong password or engine error.
 * Insertion order is preserved. Returns the new rows and how many non-PDF
 * paths were ignored.
 */
export function queuePaths(
  prev: Row[],
  paths: string[],
): { rows: Row[]; ignored: number } {
  const pdfs = paths.filter((p) => p.toLowerCase().endsWith(".pdf"));
  const byPath = new Map(prev.map((r) => [r.path, r] as const));
  for (const path of pdfs) {
    const hit = byPath.get(path);
    byPath.set(
      path,
      hit
        ? { ...hit, status: "ready", detail: undefined, outputPath: undefined }
        : { id: crypto.randomUUID(), path, name: nameFromPath(path), status: "ready" },
    );
  }
  return { rows: [...byPath.values()], ignored: paths.length - pdfs.length };
}

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

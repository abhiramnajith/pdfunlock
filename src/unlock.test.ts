import { describe, it, expect } from "vitest";
import type { Row } from "./unlock";
import { unlockOne, errorMessage, queuePaths } from "./unlock";

describe("unlockOne", () => {
  it("maps a successful unlock to a done row", async () => {
    const invoke = async () => ({ status: "Unlocked", output_path: "/a/b-unlocked.pdf" });
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("done");
    expect(patch.outputPath).toBe("/a/b-unlocked.pdf");
  });

  it("maps NotEncrypted to a skipped row", async () => {
    const invoke = async () => ({ status: "NotEncrypted" });
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("skipped");
  });

  it("maps a thrown UnlockError to an error row without throwing", async () => {
    const invoke = async () => { throw { kind: "WrongPassword" }; };
    const patch = await unlockOne("/a/b.pdf", "pw", invoke);
    expect(patch.status).toBe("error");
    expect(patch.detail).toBe("Incorrect password");
  });
});

describe("errorMessage", () => {
  it("has friendly text per kind", () => {
    expect(errorMessage({ kind: "Corrupt" })).toMatch(/read/i);
  });

  it("never returns an empty string for an empty Engine message", () => {
    const msg = errorMessage({ kind: "Engine", message: "" });
    expect(msg.trim().length).toBeGreaterThan(0);
  });

  it("never returns empty for a whitespace-only Io message", () => {
    const msg = errorMessage({ kind: "Io", message: "   " });
    expect(msg.trim().length).toBeGreaterThan(0);
  });

  it("passes a plain string throw through", () => {
    expect(errorMessage("boom from IPC")).toBe("boom from IPC");
  });

  it("uses an Error's message", () => {
    expect(errorMessage(new Error("kaboom"))).toBe("kaboom");
  });

  it("falls back to a message on a shapeless object", () => {
    expect(errorMessage({ message: "raw detail" })).toBe("raw detail");
    expect(errorMessage({}).trim().length).toBeGreaterThan(0);
  });
});

describe("queuePaths", () => {
  it("adds new .pdf paths as ready rows and ignores non-PDFs", () => {
    const { rows, ignored } = queuePaths([], ["/a/x.pdf", "/a/y.PDF", "/a/z.txt"]);
    expect(rows.map((r) => r.name)).toEqual(["x.pdf", "y.PDF"]);
    expect(rows.every((r) => r.status === "ready")).toBe(true);
    expect(ignored).toBe(1);
  });

  it("re-dropping an errored path resets it to ready, in place", () => {
    const prev: Row[] = [
      { id: "1", path: "/a/x.pdf", name: "x.pdf", status: "error", detail: "boom" },
      { id: "2", path: "/a/y.pdf", name: "y.pdf", status: "done", outputPath: "/a/y-unlocked.pdf" },
    ];
    const { rows } = queuePaths(prev, ["/a/x.pdf"]);
    expect(rows.map((r) => r.path)).toEqual(["/a/x.pdf", "/a/y.pdf"]); // order preserved
    const x = rows.find((r) => r.path === "/a/x.pdf")!;
    expect(x.status).toBe("ready");
    expect(x.detail).toBeUndefined();
    expect(x.id).toBe("1"); // same row, reset — not a duplicate
  });
});

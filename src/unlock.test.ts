import { describe, it, expect } from "vitest";
import { unlockOne, errorMessage } from "./unlock";

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
});

export type UnlockOutcome =
  | { status: "Unlocked"; output_path: string }
  | { status: "NotEncrypted" };

export type UnlockError = {
  kind: "WrongPassword" | "Corrupt" | "Io" | "Engine";
  message?: string;
};

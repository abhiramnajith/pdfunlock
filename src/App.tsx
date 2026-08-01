import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Row } from "./unlock";
import { unlockOne } from "./unlock";
import "./styles.css";

function nameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

const STATUS_LABEL: Record<Row["status"], string> = {
  ready: "Ready",
  working: "Working…",
  done: "Done",
  skipped: "Skipped",
  error: "Error",
};

function App() {
  const [rows, setRows] = useState<Row[]>([]);
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const rowsRef = useRef(rows);
  rowsRef.current = rows;

  const addPaths = useCallback((paths: string[]) => {
    setRows((prev) => {
      const existing = new Set(prev.map((r) => r.path));
      const next: Row[] = [...prev];
      for (const path of paths) {
        if (!path.toLowerCase().endsWith(".pdf")) continue;
        if (existing.has(path)) continue;
        existing.add(path);
        next.push({
          id: crypto.randomUUID(),
          path,
          name: nameFromPath(path),
          status: "ready",
        });
      }
      return next;
    });
  }, []);

  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        setDragActive(true);
      } else if (event.payload.type === "drop") {
        setDragActive(false);
        addPaths(event.payload.paths);
      } else {
        setDragActive(false);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [addPaths]);

  async function browse() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    addPaths(paths);
  }

  function patchRow(id: string, patch: Partial<Row>) {
    setRows((prev) => prev.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  }

  async function unlockAll() {
    const toProcess = rowsRef.current.filter((r) => r.status === "ready");
    for (const row of toProcess) {
      patchRow(row.id, { status: "working" });
      const patch = await unlockOne(row.path, password, invoke);
      patchRow(row.id, patch);
    }
  }

  const readyCount = rows.filter((r) => r.status === "ready").length;
  const canUnlock = readyCount > 0 && password.length > 0;

  return (
    <main className="app">
      <header className="app-header">
        <h1>PDFUnlock</h1>
        <p className="subtitle">Remove passwords from PDF files, locally.</p>
      </header>

      <section
        className={`dropzone${dragActive ? " dropzone--active" : ""}`}
      >
        <p>Drag &amp; drop PDF files here</p>
        <p className="dropzone-or">or</p>
        <button type="button" onClick={browse}>
          Browse…
        </button>
      </section>

      <section className="password-row">
        <label htmlFor="password-input">Password</label>
        <div className="password-field">
          <input
            id="password-input"
            type={showPassword ? "text" : "password"}
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            placeholder="Enter PDF password"
          />
          <button
            type="button"
            className="ghost-button"
            onClick={() => setShowPassword((v) => !v)}
          >
            {showPassword ? "Hide" : "Show"}
          </button>
        </div>
        <button
          type="button"
          className="primary-button"
          disabled={!canUnlock}
          onClick={unlockAll}
        >
          Unlock all
        </button>
      </section>

      <section className="rows">
        {rows.length === 0 && (
          <p className="empty-state">No files added yet.</p>
        )}
        {rows.map((row) => (
          <div className="file-row" key={row.id}>
            <span className="file-name" title={row.path}>
              {row.name}
            </span>
            <span className={`status-pill status-pill--${row.status}`}>
              {STATUS_LABEL[row.status]}
            </span>
            {row.detail && <span className="file-detail">{row.detail}</span>}
            {row.status === "done" && row.outputPath && (
              <button
                type="button"
                className="ghost-button"
                onClick={() => revealItemInDir(row.outputPath!)}
              >
                Reveal in Finder
              </button>
            )}
          </div>
        ))}
      </section>
    </main>
  );
}

export default App;

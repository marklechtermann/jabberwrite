import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./settings.css";

type AppInfo = {
  version: string;
  repo_url: string;
  config_path: string;
  log_dir: string;
  config_error: string | null;
  status: string;
  shortcut: string;
  platform: string;
};

type Permissions = {
  microphone: boolean;
  input_monitoring: boolean;
  accessibility: boolean;
};

const MOD_ORDER = ["Ctrl", "Shift", "Alt", "Super"];
const MOD_KEYS: Record<string, string> = { Control: "Ctrl", Shift: "Shift", Alt: "Alt", Meta: "Super" };

/** Map a KeyboardEvent.code to the key name used in config.json. */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (["Space", "Enter", "Tab", "Escape", "Backspace"].includes(code)) return code;
  return null;
}

function formatShortcut(keys: Set<string>, mac: boolean): string {
  const mods = MOD_ORDER.filter((m) => keys.has(m)).map((m) => (m === "Super" && mac ? "Cmd" : m));
  const rest = [...keys].filter((k) => !MOD_ORDER.includes(k));
  return [...mods, ...rest].join("+");
}

function ShortcutField({ value, mac, onSave }: { value: string; mac: boolean; onSave: (s: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState("");
  const save = useRef(onSave);
  save.current = onSave;

  useEffect(() => {
    if (!recording) return;
    invoke("set_hotkey_paused", { paused: true });
    const pressed = new Set<string>();

    const finish = (shortcut: string | null) => {
      setRecording(false);
      setPreview("");
      if (shortcut) save.current(shortcut);
    };
    const down = (e: KeyboardEvent) => {
      e.preventDefault();
      const name = MOD_KEYS[e.key] ?? keyName(e.code);
      if (!name) return;
      if (name === "Escape" && pressed.size === 0) return finish(null);
      // Only one non-modifier key is allowed.
      if (!MOD_ORDER.includes(name)) [...pressed].filter((k) => !MOD_ORDER.includes(k)).forEach((k) => pressed.delete(k));
      pressed.add(name);
      setPreview(formatShortcut(pressed, mac));
    };
    // Finish on the first release: macOS swallows keyup events while Cmd is held.
    const up = (e: KeyboardEvent) => {
      e.preventDefault();
      if (pressed.size > 0) finish(formatShortcut(pressed, mac));
    };

    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);
    return () => {
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
      invoke("set_hotkey_paused", { paused: false });
    };
  }, [recording, mac]);

  return (
    <div className="row">
      <code className={`shortcut ${recording ? "active" : ""}`}>
        {recording ? preview || "Press keys…" : value}
      </code>
      <button onClick={() => setRecording(!recording)}>{recording ? "Cancel" : "Record"}</button>
    </div>
  );
}

function Link({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        invoke("open_url", { url: href });
      }}
    >
      {children}
    </a>
  );
}

function PermissionRow({ label, kind, granted }: { label: string; kind: string; granted: boolean }) {
  return (
    <div className="row">
      <span>
        {granted ? "✅" : "⚠️"} {label}
      </span>
      {!granted && <button onClick={() => invoke("open_permission_settings", { kind })}>Open System Settings</button>}
    </div>
  );
}

function Settings() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [perms, setPerms] = useState<Permissions | null>(null);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  const refresh = () => {
    invoke<AppInfo>("app_info").then(setInfo);
    invoke<Permissions | null>("permissions").then(setPerms);
  };

  useEffect(() => {
    refresh();
    const unlistenStatus = listen<string>("status", (e) => setInfo((i) => i && { ...i, status: e.payload }));
    const unlistenFocus = getCurrentWindow().onFocusChanged(({ payload }) => payload && refresh());
    return () => {
      unlistenStatus.then((f) => f());
      unlistenFocus.then((f) => f());
    };
  }, []);

  if (!info) return null;
  const mac = info.platform === "macos";

  const saveShortcut = (shortcut: string) =>
    invoke<string>("save_shortcut", { shortcut })
      .then((saved) => {
        setInfo({ ...info, shortcut: saved });
        setMessage({ text: `Shortcut set to ${saved}`, error: false });
      })
      .catch((e) => setMessage({ text: String(e), error: true }));

  return (
    <main>
      <h1>Jabberwrite</h1>
      <p className="muted">Status: {info.status}</p>

      {info.config_error && <p className="error">Config file is invalid, using defaults: {info.config_error}</p>}

      {perms && !(perms.microphone && perms.input_monitoring && perms.accessibility) && (
        <section>
          <h2>Permissions</h2>
          <p className="muted">Jabberwrite needs these permissions. Restart the app after granting them.</p>
          <PermissionRow label="Microphone" kind="microphone" granted={perms.microphone} />
          <PermissionRow label="Input Monitoring (shortcut)" kind="input_monitoring" granted={perms.input_monitoring} />
          <PermissionRow label="Accessibility (paste)" kind="accessibility" granted={perms.accessibility} />
        </section>
      )}

      <section>
        <h2>Shortcut</h2>
        <p className="muted">Press once to start recording, again to stop.</p>
        <ShortcutField value={info.shortcut} mac={mac} onSave={saveShortcut} />
        {message && <p className={message.error ? "error" : "muted"}>{message.text}</p>}
      </section>

      <section>
        <h2>Files</h2>
        <label>Config file</label>
        <div className="row">
          <code className="path">{info.config_path}</code>
          <button onClick={() => invoke("open_path", { kind: "config" })}>Open folder</button>
        </div>
        <label>Logs</label>
        <div className="row">
          <code className="path">{info.log_dir}</code>
          <button onClick={() => invoke("open_path", { kind: "logs" })}>Open folder</button>
        </div>
      </section>

      <section>
        <h2>About</h2>
        <p>
          Version <code>{info.version}</code>
        </p>
        <p>
          <Link href={info.repo_url}>{info.repo_url}</Link>
        </p>
        <p className="muted">
          Speech recognition: <Link href="https://github.com/Oruk-AI/orukeet">Orukeet</Link>, based on{" "}
          <Link href="https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3">NVIDIA Parakeet TDT 0.6B v3</Link>. Model
          weights licensed under <Link href="https://creativecommons.org/licenses/by-sa/4.0/">CC BY-SA 4.0</Link>.
          Runtime: <Link href="https://github.com/k2-fsa/sherpa-onnx">sherpa-onnx</Link>.
        </p>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Settings />);

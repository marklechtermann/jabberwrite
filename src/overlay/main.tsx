import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

// Mirrors `overlay::State` in src-tauri/src/overlay.rs.
type View =
  | { state: "downloading"; progress: number }
  | { state: "loading" | "recording" | "processing" | "error" };

function Overlay() {
  const [view, setView] = useState<View | null>(null);

  useEffect(() => {
    // Events sent before this listener exists are lost, so read the current
    // state once; an event that arrives first is newer and wins.
    let heard = false;
    const unlisten = listen<View>("overlay-state", (e) => {
      heard = true;
      setView(e.payload);
    });
    invoke<View | null>("overlay_state").then((v) => {
      if (!heard && v) setView(v);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  if (!view) return null;
  const { state } = view;

  return (
    <div className={`badge ${state}`} title={state}>
      {(state === "recording" || state === "loading") && (
        <svg className="ring" viewBox="0 0 56 56">
          <circle cx="28" cy="28" r="25" pathLength="100" />
        </svg>
      )}
      {state === "recording" && (
        <svg className="glyph mic" viewBox="0 0 24 24">
          <rect x="9" y="3" width="6" height="11" rx="3" />
          <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" />
        </svg>
      )}
      {state === "loading" && (
        <svg className="glyph hourglass" viewBox="0 0 24 24">
          <path d="M7 4h10l-5 8 5 8H7l5-8z" />
        </svg>
      )}
      {state === "downloading" && (
        <>
          <svg className="progress" viewBox="0 0 56 56">
            <circle className="track" cx="28" cy="28" r="25" />
            {view.progress > 0 && (
              <circle
                className="bar"
                cx="28"
                cy="28"
                r="25"
                pathLength="100"
                strokeDasharray={`${Math.min(view.progress, 1) * 100} 100`}
              />
            )}
          </svg>
          <svg className="glyph arrow" viewBox="0 0 24 24">
            <path d="M12 4v11M7 10l5 5 5-5M5 20h14" />
          </svg>
        </>
      )}
      {state === "processing" && (
        <div className="dots">
          <i />
          <i />
          <i />
        </div>
      )}
      {state === "error" && <div className="error">!</div>}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<Overlay />);

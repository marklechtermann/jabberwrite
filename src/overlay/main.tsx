import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

type State = "recording" | "processing" | "error";

function Overlay() {
  const [state, setState] = useState<State>("recording");

  useEffect(() => {
    const unlisten = listen<State>("overlay-state", (e) => setState(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className={`badge ${state}`} title={state}>
      {state === "recording" && (
        <>
          <svg className="ring" viewBox="0 0 56 56">
            <circle cx="28" cy="28" r="25" pathLength="100" />
          </svg>
          <svg className="mic" viewBox="0 0 24 24">
            <rect x="9" y="3" width="6" height="11" rx="3" />
            <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" />
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

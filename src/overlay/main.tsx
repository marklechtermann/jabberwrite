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
      {state === "recording" && <div className="dot" />}
      {state === "processing" && <div className="spinner" />}
      {state === "error" && <div className="error">!</div>}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<Overlay />);

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import * as tauri from "../services/tauri";

export function useNativeShellSync(syncKey?: string | null) {
  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (disposed) return;
      try {
        const status = await tauri.getNativeShellStatus();
        if (!status.available || disposed) return;
        await tauri.refreshNativeShell();
      } catch (error) {
        console.error("Failed to refresh native shell snapshot:", error);
      }
    };

    void refresh();

    const subscriptions = [
      "session://started",
      "session://finished",
      "airlock:approval_required",
      "airlock:approval_resolved",
    ].map((eventName) => listen(eventName, () => void refresh()));

    return () => {
      disposed = true;
      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => unlisten());
      }
    };
  }, [syncKey]);
}

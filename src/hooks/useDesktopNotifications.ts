import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { toast } from "sonner";
import { ensureDefaultLocalAgent } from "../services/tauri";

interface DesktopNotificationRequest {
  title: string;
  body: string;
  kind: string;
  commandId?: string | null;
}

async function ensureNotificationPermission(): Promise<boolean> {
  let granted = await isPermissionGranted();
  if (!granted) {
    granted = (await requestPermission()) === "granted";
  }
  return granted;
}

export function useDesktopNotifications() {
  useEffect(() => {
    void ensureDefaultLocalAgent().catch((error) => {
      console.error("Failed to ensure default local agent:", error);
    });

    let isDisposed = false;

    const unlistenPromise = listen<DesktopNotificationRequest>(
      "desktop:notification",
      async (event) => {
        if (isDisposed) return;

        try {
          const granted = await ensureNotificationPermission();
          if (!granted) {
            if (event.payload.kind === "airlock") {
              toast.warning("Airlock approval pending", {
                description:
                  "Notifications are blocked on macOS. Open Rainy MaTE to review the request.",
              });
            }
            return;
          }

          await sendNotification({
            title: event.payload.title,
            body: event.payload.body,
          });
        } catch (error) {
          console.error("Failed to deliver desktop notification:", error);
          if (event.payload.kind === "airlock") {
            toast.error("Airlock notification failed", {
              description:
                "The approval is still pending in-app, but macOS notification delivery failed.",
            });
          }
        }
      },
    );

    return () => {
      isDisposed = true;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);
}

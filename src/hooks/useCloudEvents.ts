import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
// import { addToast } from "@heroui/react"; // Not available

interface DeployRequestPayload {
  specId: string;
  downloadUrl: string;
}

export function useCloudEvents() {
  useEffect(() => {
    const unlistenPromise = listen<DeployRequestPayload>(
      "cloud:deploy-request",
      (event) => {
        console.log("[Cloud] Deploy Request:", event.payload);

        // TODO: Implement valid toast notification or use available toast library
        /*
      addToast({
        title: "Cloud Deployment Request",
        description: `Received request to deploy agent ${event.payload.specId}`,
        color: "primary",
        timeout: 5000,
      });
      */

        // TODO: Automatically trigger download/installation logic here
        // For now, just notify.
      },
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);
}

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

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

        toast.info("Cloud Deployment Request", {
          description: `Received request to deploy agent ${event.payload.specId}`,
          duration: 5000,
          action: {
            label: "View",
            onClick: () => console.log("View deployment"),
          },
        });

        // TODO: Automatically trigger download/installation logic here
        // For now, just notify.
      },
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);
}

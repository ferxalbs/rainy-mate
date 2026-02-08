import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ensureAtmCredentialsLoaded, hasAtmCredentials } from "../services/tauri";

export interface CloudBridgeStatus {
  connected: boolean;
  mode: string;
  message: string;
}

const DEFAULT_STATUS: CloudBridgeStatus = {
  connected: false,
  mode: "http_poll",
  message: "Checking Rainy-ATM bridge...",
};

export function useCloudBridgeStatus() {
  const [status, setStatus] = useState<CloudBridgeStatus>(DEFAULT_STATUS);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const bind = async () => {
      try {
        await ensureAtmCredentialsLoaded();
      } catch {
        // No-op: we still continue with status inference.
      }

      try {
        const credentialsPresent = await hasAtmCredentials();
        if (!credentialsPresent) {
          setStatus({
            connected: false,
            mode: "http_poll",
            message: "Waiting for Rainy-ATM credentials",
          });
        }
      } catch {
        setStatus({
          connected: false,
          mode: "http_poll",
          message: "Unable to verify Rainy-ATM credentials",
        });
      }

      unlisten = await listen<CloudBridgeStatus>(
        "cloud:connection-status",
        (event) => {
          setStatus(event.payload);
        },
      );
    };

    bind();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  return status;
}

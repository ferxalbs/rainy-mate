export const CURRENT_STORAGE_KEYS = {
  theme: "rainy-mate-theme",
  mode: "rainy-mate-mode",
  animations: "rainy-mate-animations",
  compact: "rainy-mate-compact",
  chatTelemetryChips: "rainy-mate-chat-telemetry-chips",
} as const;

export function getStoredValue(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function setStoredValue(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Ignore storage failures in restricted browser contexts.
  }
}

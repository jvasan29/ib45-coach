export const DISPLAY_SCALE_STORAGE_KEY = "ib45coach.displayScale";

export const DISPLAY_SCALE_OPTIONS = [
  { value: 1, label: "Compact", description: "100%" },
  { value: 1.15, label: "Comfortable", description: "115%" },
  { value: 1.3, label: "Large", description: "130%" },
  { value: 1.5, label: "Extra large", description: "150%" },
] as const;

export type DisplayScale = (typeof DISPLAY_SCALE_OPTIONS)[number]["value"];

export function isDisplayScale(value: number): value is DisplayScale {
  return DISPLAY_SCALE_OPTIONS.some((option) => option.value === value);
}

export function readDisplayScale(storage: Pick<Storage, "getItem"> = window.localStorage): DisplayScale {
  const stored = Number(storage.getItem(DISPLAY_SCALE_STORAGE_KEY));
  return isDisplayScale(stored) ? stored : 1.15;
}

export function adjacentDisplayScale(current: DisplayScale, direction: -1 | 1): DisplayScale {
  const index = DISPLAY_SCALE_OPTIONS.findIndex((option) => option.value === current);
  const nextIndex = Math.min(DISPLAY_SCALE_OPTIONS.length - 1, Math.max(0, index + direction));
  return DISPLAY_SCALE_OPTIONS[nextIndex].value;
}

export async function applyDisplayScale(scale: DisplayScale) {
  window.localStorage.setItem(DISPLAY_SCALE_STORAGE_KEY, String(scale));
  document.documentElement.dataset.displayScale = String(Math.round(scale * 100));

  if ("__TAURI_INTERNALS__" in window) {
    try {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      await getCurrentWebview().setZoom(scale);
      document.documentElement.style.removeProperty("zoom");
      return;
    } catch {
      // CSS zoom is a safe fallback if the native permission is unavailable.
    }
  }

  // Keeps browser previews and component tests representative of the desktop app.
  document.documentElement.style.setProperty("zoom", String(scale));
}

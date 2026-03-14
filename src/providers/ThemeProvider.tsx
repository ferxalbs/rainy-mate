import {
  createContext,
  useEffect,
  useState,
  useCallback,
  useRef,
} from "react";
import { flushSync } from "react-dom";
import type { ReactNode } from "react";
import type { ThemeName, ThemeMode, ThemeConfig, Theme } from "../types/theme";
import { themes } from "../lib/themes";

const THEME_STORAGE_KEY = "rainy-cowork-theme";
const MODE_STORAGE_KEY = "rainy-cowork-mode";

interface ThemeContextType {
  theme: ThemeName;
  mode: ThemeMode;
  enableAnimations: boolean;
  config: ThemeConfig;
  setTheme: (theme: ThemeName) => void;
  setMode: (mode: ThemeMode) => void;
  setEnableAnimations: (enable: boolean) => void;
  toggleMode: () => void;
  toggleModeWithTransition: (event?: React.MouseEvent | { x: number; y: number }) => void;
  themes: Theme[];
}

export const ThemeContext = createContext<ThemeContextType | undefined>(
  undefined,
);

/**
 * Apply theme to document - can be called synchronously
 */
function applyThemeToDocument(themeName: ThemeName, themeMode: ThemeMode) {
  const selectedTheme = themes[themeName];
  if (!selectedTheme) return;

  const colors = selectedTheme.colors[themeMode];
  const root = document.documentElement;

  // Apply all color variables
  Object.entries(colors).forEach(([key, value]) => {
    const cssVar = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
    root.style.setProperty(cssVar, value);
  });

  // Apply dark class
  if (themeMode === "dark") {
    root.classList.add("dark");
  } else {
    root.classList.remove("dark");
  }

  // Set data-theme attribute
  root.setAttribute("data-theme", themeName);

  // Store in localStorage
  localStorage.setItem(THEME_STORAGE_KEY, themeName);
  localStorage.setItem(MODE_STORAGE_KEY, themeMode);
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  // Track if we've applied theme initially
  const hasAppliedInitial = useRef(false);

  const [theme, setThemeState] = useState<ThemeName>(() => {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    return (stored as ThemeName) || "rainy-tahoe";
  });

  const [mode, setModeState] = useState<ThemeMode>(() => {
    const stored = localStorage.getItem(MODE_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  });

  const [enableAnimations, setEnableAnimationsState] = useState<boolean>(() => {
    const stored = localStorage.getItem("rainy-cowork-animations");
    return stored === "true";
  });

  // Apply theme IMMEDIATELY on first render (synchronous)
  if (!hasAppliedInitial.current) {
    hasAppliedInitial.current = true;
    applyThemeToDocument(theme, mode);
  }

  // Apply theme when dependencies change (after initial)
  useEffect(() => {
    applyThemeToDocument(theme, mode);
  }, [theme, mode]);

  // Listen for system theme changes
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    const handleChange = (e: MediaQueryListEvent) => {
      const storedMode = localStorage.getItem(MODE_STORAGE_KEY);
      // Only auto-switch if user hasn't manually set a preference
      if (!storedMode) {
        setModeState(e.matches ? "dark" : "light");
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, []);

  const setTheme = useCallback((newTheme: ThemeName) => {
    setThemeState(newTheme);
  }, []);

  const setMode = useCallback((newMode: ThemeMode) => {
    setModeState(newMode);
  }, []);

  const setEnableAnimations = useCallback((enable: boolean) => {
    setEnableAnimationsState(enable);
    localStorage.setItem("rainy-cowork-animations", String(enable));
  }, []);

  const toggleMode = useCallback(() => {
    setModeState((prev) => (prev === "light" ? "dark" : "light"));
  }, []);

  const toggleModeWithTransition = useCallback((event?: React.MouseEvent | { x: number; y: number }) => {
    if (typeof document.startViewTransition !== "function") {
      toggleMode();
      return;
    }

    let x = window.innerWidth / 2;
    let y = window.innerHeight / 2;

    if (event) {
      if ("clientX" in event) {
        x = event.clientX;
        y = event.clientY;
      } else if ("x" in event) {
        x = event.x;
        y = event.y;
      }
    }

    const { width, height } = document.documentElement.getBoundingClientRect();
    const maxRadius = Math.hypot(Math.max(x, width - x), Math.max(y, height - y));

    const transition = document.startViewTransition(() => {
      flushSync(() => {
        toggleMode();
      });
    });

    transition.ready.then(() => {
      document.documentElement.animate(
        {
          clipPath: [
            `circle(0px at ${x}px ${y}px)`,
            `circle(${maxRadius}px at ${x}px ${y}px)`,
          ],
        },
        {
          duration: 550,
          easing: "cubic-bezier(0.19, 1, 0.22, 1)",
          pseudoElement: "::view-transition-new(root)",
        }
      );
    });
  }, [toggleMode]);

  const config: ThemeConfig = { theme, mode, enableAnimations };

  const value = {
    theme,
    mode,
    enableAnimations,
    config,
    setTheme,
    setMode,
    setEnableAnimations,
    toggleMode,
    toggleModeWithTransition,
    themes: Object.values(themes),
  };

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

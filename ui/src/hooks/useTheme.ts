/**
 * useTheme hook
 *
 * Manages application theme (dark/light/system).
 * Persists theme preference to localStorage and syncs with system preference.
 */

import { useState, useEffect, useCallback } from "react";

export type Theme = "dark" | "light" | "system";

const THEME_STORAGE_KEY = "jamjam-theme";

/**
 * Get the resolved theme based on system preference
 */
function getSystemTheme(): "dark" | "light" {
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/**
 * Apply theme to the document
 */
function applyTheme(theme: Theme): void {
  const root = document.documentElement;

  if (theme === "system") {
    // Remove explicit theme attribute to let CSS media query handle it
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

/**
 * Get saved theme from localStorage
 */
function getSavedTheme(): Theme {
  if (typeof window === "undefined") return "system";
  const saved = localStorage.getItem(THEME_STORAGE_KEY);
  if (saved === "dark" || saved === "light" || saved === "system") {
    return saved;
  }
  return "system";
}

/**
 * Save theme to localStorage
 */
function saveTheme(theme: Theme): void {
  if (typeof window === "undefined") return;
  localStorage.setItem(THEME_STORAGE_KEY, theme);
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(() => getSavedTheme());
  const [resolvedTheme, setResolvedTheme] = useState<"dark" | "light">(() =>
    theme === "system" ? getSystemTheme() : theme
  );

  // Apply theme on mount and when it changes
  useEffect(() => {
    applyTheme(theme);

    // Update resolved theme
    if (theme === "system") {
      setResolvedTheme(getSystemTheme());
    } else {
      setResolvedTheme(theme);
    }
  }, [theme]);

  // Listen for system theme changes when using "system" setting
  useEffect(() => {
    if (theme !== "system") return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: light)");

    const handleChange = (e: MediaQueryListEvent) => {
      setResolvedTheme(e.matches ? "light" : "dark");
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [theme]);

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme);
    saveTheme(newTheme);
  }, []);

  return {
    /** Current theme setting (dark/light/system) */
    theme,
    /** Actually applied theme (dark/light) */
    resolvedTheme,
    /** Set the theme */
    setTheme,
    /** Whether dark mode is active */
    isDark: resolvedTheme === "dark",
  };
}

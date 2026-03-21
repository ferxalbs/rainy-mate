import type { Theme, ThemeColors, ThemeMode } from '../types/theme';
import {
  CURRENT_STORAGE_KEYS,
  getStoredValue,
} from './appIdentity';

/**
 * Theme Utility Functions
 * Helper functions for working with themes
 */

/**
 * Convert hex color to OKLCH
 * Note: This is a simplified conversion, for production use a proper color library
 */
export function hexToOklch(hex: string): string {
  // This is a placeholder - in production, use a proper color conversion library
  // like culori or color.js
  return hex;
}

/**
 * Get contrast ratio between two colors
 * Used for accessibility checks
 */
export function getContrastRatio(_color1: string, _color2: string): number {
  // Placeholder - implement proper contrast calculation
  // using WCAG formula
  return 4.5;
}

/**
 * Check if a color combination meets WCAG AA standards
 */
export function meetsWCAGAA(foreground: string, background: string): boolean {
  const ratio = getContrastRatio(foreground, background);
  return ratio >= 4.5;
}

/**
 * Check if a color combination meets WCAG AAA standards
 */
export function meetsWCAGAAA(foreground: string, background: string): boolean {
  const ratio = getContrastRatio(foreground, background);
  return ratio >= 7;
}

/**
 * Validate theme colors for accessibility
 */
export function validateThemeAccessibility(colors: ThemeColors): {
  valid: boolean;
  issues: string[];
} {
  const issues: string[] = [];

  // Check text on background
  if (!meetsWCAGAA(colors.foreground, colors.background)) {
    issues.push('Foreground on background does not meet WCAG AA');
  }

  // Check card text
  if (!meetsWCAGAA(colors.cardForeground, colors.card)) {
    issues.push('Card foreground on card does not meet WCAG AA');
  }

  // Check primary button
  if (!meetsWCAGAA(colors.primaryForeground, colors.primary)) {
    issues.push('Primary foreground on primary does not meet WCAG AA');
  }

  return {
    valid: issues.length === 0,
    issues,
  };
}

/**
 * Generate CSS variables string from theme colors
 */
export function generateCSSVariables(colors: ThemeColors): string {
  return Object.entries(colors)
    .map(([key, value]) => {
      const cssVar = `--${key.replace(/([A-Z])/g, '-$1').toLowerCase()}`;
      return `${cssVar}: ${value};`;
    })
    .join('\n');
}

/**
 * Apply theme colors to document root
 */
export function applyThemeToDocument(colors: ThemeColors, mode: ThemeMode): void {
  const root = document.documentElement;

  // Apply all color variables
  Object.entries(colors).forEach(([key, value]) => {
    const cssVar = `--${key.replace(/([A-Z])/g, '-$1').toLowerCase()}`;
    root.style.setProperty(cssVar, value);
  });

  // Apply dark class
  if (mode === 'dark') {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}

/**
 * Get theme from localStorage
 */
export function getStoredTheme(): string | null {
  return getStoredValue(CURRENT_STORAGE_KEYS.theme);
}

/**
 * Get mode from localStorage
 */
export function getStoredMode(): ThemeMode | null {
  const stored = getStoredValue(CURRENT_STORAGE_KEYS.mode);
  if (stored === 'light' || stored === 'dark') return stored;
  return null;
}

/**
 * Detect system theme preference
 */
export function detectSystemTheme(): ThemeMode {
  if (typeof window === 'undefined') return 'light';

  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

/**
 * Export theme as JSON
 */
export function exportTheme(theme: Theme): string {
  return JSON.stringify(theme, null, 2);
}

/**
 * Import theme from JSON
 */
export function importTheme(json: string): Theme | null {
  try {
    const theme = JSON.parse(json);
    // Validate theme structure
    if (!theme.name || !theme.colors || !theme.colors.light || !theme.colors.dark) {
      return null;
    }
    return theme as Theme;
  } catch {
    return null;
  }
}

/**
 * Interpolate between two colors
 * Useful for creating color transitions
 */
export function interpolateColors(
  color1: string,
  color2: string,
  factor: number
): string {
  // Placeholder - implement proper color interpolation
  // in OKLCH space for smooth transitions
  return factor < 0.5 ? color1 : color2;
}

/**
 * Generate a color palette from a base color
 */
export function generatePalette(baseColor: string): {
  lighter: string;
  light: string;
  base: string;
  dark: string;
  darker: string;
} {
  // Placeholder - implement proper palette generation
  return {
    lighter: baseColor,
    light: baseColor,
    base: baseColor,
    dark: baseColor,
    darker: baseColor,
  };
}

/**
 * Check if theme is dark based on background lightness
 */
export function isThemeDark(colors: ThemeColors): boolean {
  // Extract lightness from OKLCH background color
  const match = colors.background.match(/oklch\(([\d.]+)/);
  if (!match) return false;

  const lightness = parseFloat(match[1]);
  return lightness < 0.5;
}

/**
 * Get readable color name from OKLCH value
 */
export function getColorName(_oklch: string): string {
  // Placeholder - implement color naming
  return 'Custom Color';
}

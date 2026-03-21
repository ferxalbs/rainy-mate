/**
 * Theme System Configuration
 * Centralized configuration for theme behavior and defaults
 */

import type { ThemeName, ThemeMode } from '../types/theme';
import { CURRENT_STORAGE_KEYS } from '../lib/appIdentity';

export const THEME_CONFIG = {
  /**
   * Default theme to use when no preference is saved
   */
  defaultTheme: 'cosmic-gold' as ThemeName,

  /**
   * Default mode to use when no preference is saved
   * Set to null to auto-detect from system
   */
  defaultMode: null as ThemeMode | null,

  /**
   * Storage keys for localStorage
   */
  storage: {
    theme: CURRENT_STORAGE_KEYS.theme,
    mode: CURRENT_STORAGE_KEYS.mode,
  },

  /**
   * Enable automatic system theme detection
   */
  autoDetectSystemTheme: true,

  /**
   * Enable smooth transitions when switching themes
   */
  enableTransitions: true,

  /**
   * Transition duration in milliseconds
   */
  transitionDuration: 200,

  /**
   * Enable theme persistence in localStorage
   */
  enablePersistence: true,

  /**
   * Enable debug logging
   */
  debug: false,
} as const;

/**
 * Theme feature flags
 */
export const THEME_FEATURES = {
  /**
   * Allow users to create custom themes
   */
  customThemes: false,

  /**
   * Show theme preview in selector
   */
  showPreview: true,

  /**
   * Show color palette in theme cards
   */
  showColorPalette: true,

  /**
   * Enable theme export/import
   */
  exportImport: false,

  /**
   * Show theme showcase page
   */
  showcase: true,
} as const;

/**
 * Accessibility settings
 */
export const THEME_ACCESSIBILITY = {
  /**
   * Minimum contrast ratio for text (WCAG AA)
   */
  minContrastRatio: 4.5,

  /**
   * Enable high contrast mode option
   */
  highContrastMode: false,

  /**
   * Respect prefers-reduced-motion
   */
  respectReducedMotion: true,

  /**
   * Enable focus indicators
   */
  focusIndicators: true,
} as const;

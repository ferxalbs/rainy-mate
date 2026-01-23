// Theme System Types
export type ThemeMode = 'light' | 'dark';

export type ThemeName =
  | 'cosmic-gold'
  | 'cosmic-night'
  | 'jujutsu-kaisen'
  | 'anime-style';

export interface Theme {
  name: ThemeName;
  displayName: string;
  description: string;
  icon: string;
  colors: {
    light: ThemeColors;
    dark: ThemeColors;
  };
}

export interface ThemeColors {
  background: string;
  foreground: string;
  card: string;
  cardForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryForeground: string;
  secondary: string;
  secondaryForeground: string;
  muted: string;
  mutedForeground: string;
  accent: string;
  accentForeground: string;
  destructive: string;
  destructiveForeground: string;
  border: string;
  input: string;
  ring: string;
  sidebar: string;
  sidebarForeground: string;
  sidebarPrimary: string;
  sidebarAccent: string;
  sidebarBorder: string;
}


export interface ThemeConfig {
  theme: ThemeName;
  mode: ThemeMode;
  enableAnimations: boolean;
}

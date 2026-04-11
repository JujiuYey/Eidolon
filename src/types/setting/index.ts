export type Theme = 'light' | 'dark' | 'system';
export type ThemeColor = 'emerald' | 'rose' | 'teal' | 'indigo' | 'violet' | 'pink' | 'blue' | 'amber' | 'purple' | 'sky';

// 应用设置接口
export interface AppSettings {
  theme: Theme;
  themeColor: ThemeColor;
}

export interface GitConfig {
  gitUsername: string;
  gitEmail: string;
}

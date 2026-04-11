export type Theme = 'light' | 'dark' | 'system';
export type ThemeColor = 'rose' | 'pink' | 'amber' | 'orange' | 'yellow' | 'emerald' | 'teal' | 'blue' | 'indigo' | 'purple';

// 应用设置接口
export interface AppSettings {
  theme: Theme;
  themeColor: ThemeColor;
}

export interface GitConfig {
  gitUsername: string;
  gitEmail: string;
}

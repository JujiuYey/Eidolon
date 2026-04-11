export type Theme = 'light' | 'dark' | 'system';

// 应用设置接口
export interface AppSettings {
  autoSave: boolean;
  theme: Theme;
  projectPath: string | null;
  storageDir: string | null;
  projectFilesExtensions: string;
  projectFilesIgnoreDirs: string;
  projectFilesMaxFileContentLength: number;
}

export interface GitConfig {
  gitUsername: string;
  gitEmail: string;
}

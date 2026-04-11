import { invoke } from '@tauri-apps/api/core';
import type {
  ProjectFileEntry,
  ProjectFilesSettings,
  ProjectFileSearchResult,
} from '@/types/project-files';

interface ProjectFilesSettingsPayload {
  file_extensions: string;
  ignore_dirs: string;
  max_file_content_length: number;
}

function toSettingsPayload(settings: ProjectFilesSettings): ProjectFilesSettingsPayload {
  return {
    file_extensions: settings.fileExtensions,
    ignore_dirs: settings.ignoreDirs,
    max_file_content_length: settings.maxFileContentLength,
  };
}

export async function scanProjectFiles(projectPath: string, settings: ProjectFilesSettings): Promise<ProjectFileEntry[]> {
  return invoke<ProjectFileEntry[]>('scan_project_files', {
    request: {
      project_path: projectPath,
      settings: toSettingsPayload(settings),
    },
  });
}

export async function readProjectFile(payload: {
  projectPath: string;
  filePath: string;
  startLine?: number;
  endLine?: number;
  maxChars?: number;
}): Promise<string> {
  return invoke<string>('read_project_file', {
    request: {
      project_path: payload.projectPath,
      file_path: payload.filePath,
      start_line: payload.startLine,
      end_line: payload.endLine,
      max_chars: payload.maxChars,
    },
  });
}

export async function searchProjectFiles(payload: {
  projectPath: string;
  keyword: string;
  filePattern?: string;
  contextLines?: number;
  settings: ProjectFilesSettings;
}): Promise<ProjectFileSearchResult[]> {
  return invoke<ProjectFileSearchResult[]>('search_project_files', {
    request: {
      project_path: payload.projectPath,
      keyword: payload.keyword,
      file_pattern: payload.filePattern,
      context_lines: payload.contextLines,
      settings: toSettingsPayload(payload.settings),
    },
  });
}

export async function listProjectDirectory(payload: {
  projectPath: string;
  directoryPath?: string | null;
}): Promise<ProjectFileEntry[]> {
  return invoke<ProjectFileEntry[]>('list_project_directory', {
    request: {
      project_path: payload.projectPath,
      directory_path: payload.directoryPath ?? null,
    },
  });
}

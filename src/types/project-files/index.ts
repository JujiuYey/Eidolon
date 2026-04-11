export interface ProjectFilesSettings {
  fileExtensions: string;
  ignoreDirs: string;
  maxFileContentLength: number;
}

export interface ProjectFileEntry {
  name: string;
  path: string;
  file_type: string;
}

export interface ProjectFileLineMatch {
  line_number: number;
  content: string;
  is_target: boolean;
}

export interface ProjectFileSearchResult {
  file: string;
  matches: ProjectFileLineMatch[];
}

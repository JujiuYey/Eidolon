import { invoke } from '@tauri-apps/api/core';

export interface ParsedField {
  name: string;
  goName: string;
  jsonName: string;
  goType: string;
  dbType: string;
  nullable: boolean;
  isPrimaryKey: boolean;
  defaultValue: string | null;
  comment: string | null;
  validate: string | null;
}

export interface ParsedTable {
  tableName: string;
  entityName: string;
  entityLabel: string | null;
  fields: ParsedField[];
}

export type AuditType
  = | 'fullAudited'
    | 'fullTracked'
    | 'creationAudited'
    | 'creationTracked'
    | 'none';

export interface GoCodeGenConfig {
  entityName: string;
  modulePath: string;
  tableName: string;
  tableAlias: string;
  rpcPath: string;
  goModulePrefix: string;
  outputDir: string;
  fields: ParsedField[];
  auditType: AuditType;
  enableFindPage: boolean;
  enableCreate: boolean;
  enableUpdate: boolean;
  enableDelete: boolean;
  enableDeleteMany: boolean;
  enableSort: boolean;
  enableAuditUserNames: boolean;
  overwrite: boolean;
}

export interface GeneratedGoCrudResult {
  generatedFiles: string[];
  moduleRegistrationSnippet: string;
}

export async function generateFrontendCrud(request: {
  resourcePath: string;
  frontendBasePath: string;
  overwrite?: boolean;
}) {
  return invoke<string[]>('generate_crud', request);
}

export async function parseSql(sql: string) {
  return invoke<ParsedTable>('parse_sql_ddl', { sql });
}

export async function generateGoCode(config: GoCodeGenConfig) {
  return invoke<GeneratedGoCrudResult>('generate_go_crud', { config });
}

export interface ServiceTag {
  label: string;
  class: string;
}

export interface McpServiceCard {
  id: string;
  name: string;
  enabled: boolean;
  highlighted?: boolean;
  tags: ServiceTag[];
}

export type McpServiceFormMode = 'create' | 'edit';

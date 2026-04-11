/**
 * !! 待删除
 */

/**
 * 单个 AI 服务配置
 */
export interface AIConfig {
  /** 唯一标识符 */
  id: string;
  /** 配置名称（用户自定义） */
  name: string;
  /** API 密钥 */
  api_key: string;
  /** API 基础地址 */
  base_url: string;
  /** 模型名称 */
  model: string;
}

/**
 * 全局 AI 配置
 */
export interface AISettings {
  /** 所有配置列表 */
  configs: AIConfig[];
}

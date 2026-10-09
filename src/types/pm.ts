/** 产品经理分身：会话（镜像后端 models/pm.rs，snake_case） */
export interface PmConversation {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
}

/** 会话消息。发送失败的消息不会入库，因此只有成功消息 */
export interface PmMessage {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant';
  content: string;
  /** 本轮回答实际参考的方法论框架 slug */
  skills_used: string[];
  created_at: number;
}

/** "builtin"（随应用打包）或 "override"（来自覆盖目录） */
export type PmSkillSource = 'builtin' | 'override';

/** 方法论框架清单条目（设置页展示用） */
export interface PmSkillSummary {
  slug: string;
  name: string;
  description: string;
  source: PmSkillSource;
  size: number;
}

/** 产品经理分身的功能设置 */
export interface PmSettings {
  skills_override_dir: string | null;
}

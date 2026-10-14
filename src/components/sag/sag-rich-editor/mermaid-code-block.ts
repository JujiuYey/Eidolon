import { CodeBlockLowlight } from '@tiptap/extension-code-block-lowlight';
import { VueNodeViewRenderer } from '@tiptap/vue-3';

import MermaidCodeBlockView from './mermaid-code-block-view.vue';

/**
 * 代码块扩展:换成 lowlight 底座做语法高亮,节点视图负责 mermaid 图表卡,
 * 节点名、schema、输入规则与 tiptap-markdown 序列化均保持不变。
 */
export const MermaidCodeBlock = CodeBlockLowlight.extend({
  addNodeView() {
    return VueNodeViewRenderer(MermaidCodeBlockView);
  },
});

import CodeBlock from '@tiptap/extension-code-block';
import { VueNodeViewRenderer } from '@tiptap/vue-3';
import MermaidCodeBlockView from './mermaid-code-block-view.vue';

/**
 * 代码块扩展:仅替换节点视图(mermaid 语言时附图表预览),
 * 节点名、schema、输入规则与 tiptap-markdown 序列化均保持不变。
 */
export const MermaidCodeBlock = CodeBlock.extend({
  addNodeView() {
    return VueNodeViewRenderer(MermaidCodeBlockView);
  },
});

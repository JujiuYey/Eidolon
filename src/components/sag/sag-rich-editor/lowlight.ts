import bash from 'highlight.js/lib/languages/bash';
import css from 'highlight.js/lib/languages/css';
import diff from 'highlight.js/lib/languages/diff';
import go from 'highlight.js/lib/languages/go';
import ini from 'highlight.js/lib/languages/ini';
import java from 'highlight.js/lib/languages/java';
import javascript from 'highlight.js/lib/languages/javascript';
import json from 'highlight.js/lib/languages/json';
import markdown from 'highlight.js/lib/languages/markdown';
import properties from 'highlight.js/lib/languages/properties';
import python from 'highlight.js/lib/languages/python';
import sql from 'highlight.js/lib/languages/sql';
import typescript from 'highlight.js/lib/languages/typescript';
import xml from 'highlight.js/lib/languages/xml';
import yaml from 'highlight.js/lib/languages/yaml';
import { createLowlight } from 'lowlight';

const lowlight = createLowlight();

// 按需注册控制打包体积，别引 common 全家桶；别名（js/html/yml/sh…）随主名自动生效
lowlight.register({
  bash,
  css,
  diff,
  go,
  ini,
  java,
  javascript,
  json,
  markdown,
  properties,
  python,
  sql,
  typescript,
  xml,
  yaml,
});

/**
 * 给 CodeBlockLowlight 用的接口面。原版行为是语言没注册就 highlightAuto 自动猜
 * ——mermaid 块、没标语言的块会被胡乱上色还慢，这里收掉：未注册一律纯文本；
 * highlight 出错也降级为纯文本，不能打断编辑事务。
 */
export const editorLowlight = {
  highlight(language: string, value: string) {
    try {
      return lowlight.highlight(language, value);
    } catch {
      return { children: [] };
    }
  },
  highlightAuto() {
    return { children: [] };
  },
  listLanguages: () => lowlight.listLanguages(),
  registered: (language: string) => lowlight.registered(language),
};

/**
 * 邮件模板变量工具（纯函数）。
 *
 * 模板语法：`{{变量名}}`，支持中文变量名与花括号内侧空格（`{{ 姓名 }}`）。
 * 变量提取、渲染全部在此完成，后端只接收渲染后的最终文本。
 */

const VARIABLE_PATTERN = /\{\{\s*([^{}]+?)\s*\}\}/g;

/**
 * 从主题与正文中提取变量名列表：去重、按首次出现顺序返回。
 */
export function extractVariables(subject: string, body: string): string[] {
  const variables: string[] = [];
  const seen = new Set<string>();

  for (const text of [subject, body]) {
    for (const match of text.matchAll(VARIABLE_PATTERN)) {
      const name = (match[1] ?? '').trim();
      if (!name || seen.has(name)) {
        continue;
      }
      seen.add(name);
      variables.push(name);
    }
  }

  return variables;
}

/**
 * 用变量值渲染模板文本。未提供或为空值的变量保持原占位符不动，
 * 便于用户在预览中看到尚缺的内容。
 */
export function renderTemplate(
  text: string,
  values: Record<string, string>,
): string {
  return text.replace(VARIABLE_PATTERN, (placeholder, rawName: string) => {
    const name = rawName.trim();
    const value = values[name];
    return value || placeholder;
  });
}

/**
 * 返回尚未填写（空白）的变量名列表，用于发送前校验。
 */
export function missingVariables(
  variables: string[],
  values: Record<string, string>,
): string[] {
  return variables.filter(name => (values[name] ?? '').trim() === '');
}

/**
 * 根据变量列表生成空白值表。
 */
export function buildEmptyValues(variables: string[]): Record<string, string> {
  return Object.fromEntries(variables.map(name => [name, '']));
}

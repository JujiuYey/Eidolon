import { describe, expect, it } from 'vitest';
import {
  buildEmptyValues,
  extractVariables,
  missingVariables,
  renderTemplate,
} from '../template-helpers';

describe('extractVariables', () => {
  it('提取主题与正文中的变量并去重、保序', () => {
    const variables = extractVariables(
      '{{姓名}}的{{请假类型}}申请',
      '尊敬的{{审批人}}：我因{{请假事由}}请假{{请假天数}}天。{{审批人}}请查收。',
    );

    expect(variables).toEqual([
      '姓名',
      '请假类型',
      '审批人',
      '请假事由',
      '请假天数',
    ]);
  });

  it('兼容花括号内侧空格与英文变量名', () => {
    expect(extractVariables('{{ name }}的{{ type }}申请', '')).toEqual([
      'name',
      'type',
    ]);
  });

  it('同一变量在两个文本间共享时只出现一次', () => {
    expect(extractVariables('{{ 姓名 }}的请假申请', '{{姓名}} {{日期}}')).toEqual([
      '姓名',
      '日期',
    ]);
  });

  it('无变量时返回空数组', () => {
    expect(extractVariables('普通主题', '普通正文')).toEqual([]);
    expect(extractVariables('', '')).toEqual([]);
  });

  it('忽略空变量与嵌套花括号', () => {
    expect(extractVariables('{{}} {{ }}', '{{a{b}c}}')).toEqual([]);
  });
});

describe('renderTemplate', () => {
  it('替换已提供的变量', () => {
    const rendered = renderTemplate('{{姓名}}的{{请假类型}}申请', {
      姓名: '张三',
      请假类型: '病假',
    });

    expect(rendered).toBe('张三的病假申请');
  });

  it('未提供的变量保持占位符', () => {
    const rendered = renderTemplate('{{姓名}}的{{请假类型}}申请', {
      姓名: '张三',
    });

    expect(rendered).toBe('张三的{{请假类型}}申请');
  });

  it('替换值中包含 $ 等特殊字符时按字面处理', () => {
    const rendered = renderTemplate('金额：{{金额}}', { 金额: '$&$1$$' });

    expect(rendered).toBe('金额：$&$1$$');
  });

  it('值首尾空白保留原样（不做裁剪）', () => {
    const rendered = renderTemplate('{{内容}}', { 内容: ' 前后空格 ' });

    expect(rendered).toBe(' 前后空格 ');
  });

  it('空值表渲染时全部占位符保持原样', () => {
    const text = '{{姓名}}的{{日期}}周报';
    expect(renderTemplate(text, {})).toBe(text);
  });
});

describe('missingVariables', () => {
  it('返回未填写或仅空白的变量', () => {
    const missing = missingVariables(['姓名', '日期', '事由'], {
      姓名: '张三',
      日期: '  ',
    });

    expect(missing).toEqual(['日期', '事由']);
  });

  it('全部填写时返回空数组', () => {
    expect(missingVariables(['姓名'], { 姓名: '张三' })).toEqual([]);
    expect(missingVariables([], {})).toEqual([]);
  });
});

describe('buildEmptyValues', () => {
  it('按变量列表生成空白值表', () => {
    expect(buildEmptyValues(['姓名', '日期'])).toEqual({ 姓名: '', 日期: '' });
    expect(buildEmptyValues([])).toEqual({});
  });
});

import type { JavaManifest } from '../types';

/**
 * 按 Rust code_scan_java 的输出形状手工构造的样本：
 * 继承基类的业务 Controller（含声明端点）+ 独立 Controller + 表单字段表。
 */
export const JAVA_MANIFEST_FIXTURE: JavaManifest = {
  framework: 'vef-java',
  controllers: [
    {
      name: 'DepartmentController',
      module: 'system',
      base_path: '/system/department',
      extends_base: true,
      update_form: 'DepartmentUpdateForm',
      search_form: 'DepartmentSearchForm',
      endpoints: [
        {
          name: 'fetchPage',
          method: 'GET',
          path: '/page',
          source: 'base',
          body_form: null,
          query_params: [],
        },
        {
          name: 'create',
          method: 'POST',
          path: '/',
          source: 'base',
          body_form: 'DepartmentUpdateForm',
          query_params: [],
        },
        {
          name: 'batchCreate',
          method: 'POST',
          path: '/batch',
          source: 'base',
          body_form: 'DepartmentUpdateForm',
          query_params: [],
        },
        {
          name: 'fetchOne',
          method: 'GET',
          path: '/',
          source: 'base',
          body_form: null,
          query_params: [{ name: 'id', required: true }],
        },
        {
          name: 'fetchDepartmentTree',
          method: 'GET',
          path: '/department-tree',
          source: 'declared',
          body_form: null,
          query_params: [{ name: 'tenantId', required: false }],
        },
      ],
    },
    {
      name: 'ExternalApiController',
      module: 'api',
      base_path: '/external-api',
      extends_base: false,
      update_form: null,
      search_form: null,
      endpoints: [
        {
          name: 'syncBusinessData',
          method: 'POST',
          path: '/business-data',
          source: 'declared',
          body_form: 'ExternalRequestDto',
          query_params: [],
        },
      ],
    },
  ],
  forms: {
    DepartmentUpdateForm: {
      fields: [
        { name: 'name', field_type: 'String', required: true, label: '部门名称' },
        { name: 'type', field_type: 'DepartmentType', required: true, label: '部门类型' },
        { name: 'remark', field_type: 'String', required: false, label: null },
      ],
    },
    DepartmentSearchForm: {
      fields: [
        { name: 'status', field_type: 'SwitchStatus', required: false, label: null },
        { name: 'name', field_type: 'String', required: false, label: '部门名称' },
      ],
    },
    ExternalRequestDto: {
      fields: [{ name: 'data', field_type: 'List<Object>', required: true, label: null }],
    },
  },
};

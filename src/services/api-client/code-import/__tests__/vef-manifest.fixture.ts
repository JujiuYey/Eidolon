import type { VefManifest } from '../types';

/**
 * 从 smp-server 真实导出清单裁剪的样本：覆盖业务资源（smp/sys/user）、
 * 框架内置资源（security/auth）与常见字段类型（required、optional、
 * interface、[]interface）。
 */
export const SMP_MANIFEST_FIXTURE: VefManifest = {
  framework: 'v0.56.0',
  resources: [
    {
      name: 'security/auth',
      kind: 'rpc',
      version: 'v1',
      operations: [
        {
          action: 'get_user_info',
          auth: 'bearer',
          timeoutMs: 30000,
          rateLimit: { max: 100, periodMs: 300000 },
        },
        {
          action: 'login',
          auth: 'none',
          timeoutMs: 30000,
          rateLimit: { max: 6, periodMs: 300000 },
          params: 'github.com/coldsmirk/vef-framework-go/internal/security.LoginParams',
        },
        {
          action: 'refresh',
          auth: 'none',
          timeoutMs: 30000,
          rateLimit: { max: 1, periodMs: 300000 },
          params: 'github.com/coldsmirk/vef-framework-go/internal/security.RefreshParams',
        },
      ],
    },
    {
      name: 'smp/sys/user',
      kind: 'rpc',
      version: 'v1',
      operations: [
        {
          action: 'change_password',
          auth: 'bearer',
          timeoutMs: 30000,
          params: 'smp-server/internal/sys/payload.UserChangePasswordParams',
        },
        {
          action: 'delete_many',
          auth: 'bearer',
          permission: 'sys.user.delete_many',
          audit: true,
          timeoutMs: 30000,
          params: 'github.com/coldsmirk/vef-framework-go/crud.DeleteManyParams',
        },
        {
          action: 'find_page',
          auth: 'bearer',
          permission: 'sys.user.query',
          timeoutMs: 30000,
          params: 'smp-server/internal/sys/payload.UserSearch',
        },
      ],
    },
    {
      name: 'smp/sys/empty',
      kind: 'rpc',
      version: 'v1',
      operations: [],
    },
  ],
  types: {
    'github.com/coldsmirk/vef-framework-go/internal/security.LoginParams': {
      fields: [
        { name: 'type', type: 'string', validate: 'required' },
        { name: 'principal', type: 'string', validate: 'required' },
        { name: 'credentials', type: 'interface', validate: 'required' },
      ],
    },
    'github.com/coldsmirk/vef-framework-go/internal/security.RefreshParams': {
      fields: [{ name: 'refreshToken', type: 'string', validate: 'required' }],
    },
    'github.com/coldsmirk/vef-framework-go/crud.DeleteManyParams': {
      fields: [{ name: 'pks', type: '[]interface', validate: 'required,min=1' }],
    },
    'smp-server/internal/sys/payload.UserSearch': {
      fields: [
        { name: 'keyword', type: 'string' },
        {
          name: 'organizationId',
          type: 'string',
          optional: true,
          label: '机构ID',
          validate: 'omitempty,alphanum,max=32',
        },
        { name: 'phoneNumber', type: 'string', optional: true },
      ],
    },
    'smp-server/internal/sys/payload.UserChangePasswordParams': {
      fields: [
        { name: 'oldPassword', type: 'string', label: '旧密码', validate: 'required,min=6,max=16,ascii' },
        { name: 'newPassword', type: 'string', label: '新密码', validate: 'required,min=6,max=16,ascii' },
        { name: 'confirmPassword', type: 'string', label: '确认密码', validate: 'required,min=6,max=16,ascii' },
      ],
    },
  },
};

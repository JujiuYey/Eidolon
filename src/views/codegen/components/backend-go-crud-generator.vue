<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { Copy, FolderOpen, RefreshCcw, Sparkles } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import type { GoCodeGenConfig, ParsedTable } from '@/services/codegen';
import { generateGoCode, parseSql } from '@/services/codegen';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { copyToClipboard, getErrorMessage } from '@/utils/helpers';

const sql = ref('');
const parsing = ref(false);
const generating = ref(false);
const error = ref('');
const parsedTable = ref<ParsedTable | null>(null);
const generatedFiles = ref<string[]>([]);
const moduleRegistrationSnippet = ref('');

const config = reactive<GoCodeGenConfig>({
  entityName: '',
  modulePath: 'sys',
  tableName: '',
  tableAlias: '',
  rpcPath: '',
  goModulePrefix: 'smp-server/internal',
  outputDir: '',
  fields: [],
  auditType: 'fullAudited',
  enableFindPage: true,
  enableCreate: true,
  enableUpdate: true,
  enableDelete: true,
  enableDeleteMany: false,
  enableSort: false,
  enableAuditUserNames: true,
  overwrite: false,
});

const canParse = computed(() => sql.value.trim().length > 0 && !parsing.value);
const canGenerate = computed(() => {
  return !generating.value
    && config.fields.length > 0
    && config.entityName.trim().length > 0
    && config.tableName.trim().length > 0
    && config.tableAlias.trim().length > 0
    && config.modulePath.trim().length > 0
    && config.rpcPath.trim().length > 0
    && config.goModulePrefix.trim().length > 0
    && config.outputDir.trim().length > 0
    && (config.enableFindPage || config.enableCreate || config.enableUpdate || config.enableDelete || config.enableDeleteMany);
});

watch(() => config.modulePath, modulePath => {
  config.enableAuditUserNames = modulePath.trim().split('/')[0] === 'sys';
  if (parsedTable.value) {
    config.rpcPath = buildRpcPath(modulePath, config.entityName);
  }
});

watch(() => config.entityName, entityName => {
  config.tableAlias = deriveAlias(entityName);
  if (parsedTable.value) {
    config.rpcPath = buildRpcPath(config.modulePath, entityName);
  }
});

async function handleParse() {
  if (!canParse.value) {
    return;
  }

  parsing.value = true;
  error.value = '';
  generatedFiles.value = [];
  moduleRegistrationSnippet.value = '';

  try {
    const result = await parseSql(sql.value);
    parsedTable.value = result;
    applyParsedTable(result);
    toast.success(`SQL 解析完成，识别到 ${result.fields.length} 个业务字段`);
  } catch (err) {
    parsedTable.value = null;
    config.fields = [];
    error.value = getErrorMessage(err, 'SQL 解析失败');
    toast.error(error.value);
  } finally {
    parsing.value = false;
  }
}

function applyParsedTable(table: ParsedTable) {
  config.fields = table.fields;
  config.entityName = table.entityName;
  config.tableName = table.tableName;
  config.tableAlias = deriveAlias(table.entityName);
  config.enableSort = table.fields.some(field => field.name === 'sort_order');
  config.enableAuditUserNames = config.modulePath.trim().split('/')[0] === 'sys';
  config.rpcPath = buildRpcPath(config.modulePath, table.entityName);
}

function deriveAlias(entityName: string) {
  const initials = entityName.match(/[A-Z]/g)?.join('').toLowerCase() ?? '';
  return initials || entityName.slice(0, 1).toLowerCase();
}

function buildRpcPath(modulePath: string, entityName: string) {
  const normalizedModule = modulePath
    .trim()
    .replace(/^\/+|\/+$/g, '')
    .replace(/\/{2,}/g, '/');

  const entityPath = toSnakeCase(entityName);
  if (!normalizedModule || !entityPath) {
    return '';
  }

  return `smp/${normalizedModule}/${entityPath}`;
}

function toSnakeCase(value: string) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .replace(/[-\s]+/g, '_')
    .toLowerCase();
}

async function selectOutputDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择 Go 模块输出目录',
  });

  if (selected && typeof selected === 'string') {
    config.outputDir = selected;
    error.value = '';
  }
}

async function handleGenerate() {
  if (!canGenerate.value) {
    toast.error('请先完成 SQL 解析并补齐生成配置');
    return;
  }

  generating.value = true;
  error.value = '';
  generatedFiles.value = [];
  moduleRegistrationSnippet.value = '';

  try {
    const result = await generateGoCode({ ...config, fields: [...config.fields] });
    generatedFiles.value = result.generatedFiles;
    moduleRegistrationSnippet.value = result.moduleRegistrationSnippet;
    toast.success(`生成完成，共输出 ${result.generatedFiles.length} 个文件`);
  } catch (err) {
    error.value = getErrorMessage(err, 'Go CRUD 生成失败');
    toast.error(error.value);
  } finally {
    generating.value = false;
  }
}

async function handleCopySnippet() {
  if (!moduleRegistrationSnippet.value) {
    return;
  }

  const copied = await copyToClipboard(moduleRegistrationSnippet.value);
  if (copied) {
    toast.success('module.go 注册代码已复制');
  }
}

function resetSql() {
  sql.value = '';
  error.value = '';
  parsedTable.value = null;
  generatedFiles.value = [];
  moduleRegistrationSnippet.value = '';
  config.entityName = '';
  config.tableName = '';
  config.tableAlias = '';
  config.rpcPath = '';
  config.outputDir = '';
  config.fields = [];
  config.enableSort = false;
}
</script>

<template>
  <div class="space-y-6">
    <Alert>
      <AlertTitle>后端 Go CRUD MVP</AlertTitle>
      <AlertDescription>
        当前版本优先保证 SQL 解析、Go 文件生成、覆盖保护和 `module.go` 注册提示可用，目录树懒加载与代码预览暂未接入。
      </AlertDescription>
    </Alert>

    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Sparkles class="h-4 w-4 text-primary" />
          1. SQL DDL 输入
        </CardTitle>
        <CardDescription>
          粘贴 MySQL 风格 `CREATE TABLE` 语句，支持列 `COMMENT` 和常见基础类型映射。
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <Textarea
          v-model="sql"
          placeholder="CREATE TABLE sys_user (...);"
          class="min-h-64 font-mono text-sm"
        />

        <div class="flex flex-wrap gap-3">
          <Button :disabled="!canParse" @click="handleParse">
            {{ parsing ? '解析中...' : '解析 SQL' }}
          </Button>
          <Button variant="outline" @click="resetSql">
            <RefreshCcw class="mr-2 h-4 w-4" />
            清空
          </Button>
        </div>
      </CardContent>
    </Card>

    <Card v-if="parsedTable">
      <CardHeader>
        <CardTitle>2. 解析结果</CardTitle>
        <CardDescription>
          表名 `{{ parsedTable.tableName }}`，实体名 `{{ parsedTable.entityName }}`，共 {{ parsedTable.fields.length }} 个业务字段。
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="grid gap-3 lg:grid-cols-2">
          <div class="rounded-lg border bg-muted/30 p-4">
            <p class="text-sm font-medium">
              实体标签
            </p>
            <p class="mt-1 text-sm text-muted-foreground">
              {{ parsedTable.entityLabel || '未提供表注释，将使用实体名作为默认说明。' }}
            </p>
          </div>
          <div class="rounded-lg border bg-muted/30 p-4">
            <p class="text-sm font-medium">
              默认排序字段
            </p>
            <p class="mt-1 text-sm text-muted-foreground">
              {{ config.enableSort ? '检测到 sort_order，将默认启用排序。' : '未检测到 sort_order，将不生成默认排序链路。' }}
            </p>
          </div>
        </div>

        <div class="rounded-lg border">
          <div class="grid grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)_minmax(0,1fr)] gap-3 border-b bg-muted/30 px-4 py-3 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            <span>字段</span>
            <span>Go 类型</span>
            <span>校验 / 注释</span>
          </div>
          <ul class="divide-y text-sm">
            <li
              v-for="field of parsedTable.fields"
              :key="field.name"
              class="grid grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)_minmax(0,1fr)] gap-3 px-4 py-3"
            >
              <div class="min-w-0">
                <p class="truncate font-medium">
                  {{ field.name }}
                </p>
                <p class="truncate font-mono text-xs text-muted-foreground">
                  {{ field.goName }} / {{ field.jsonName }}
                </p>
              </div>
              <div class="font-mono text-xs text-muted-foreground">
                {{ field.goType }}
              </div>
              <div class="min-w-0 text-xs text-muted-foreground">
                <p class="truncate">
                  {{ field.validate || '无额外校验' }}
                </p>
                <p class="truncate">
                  {{ field.comment || '无注释' }}
                </p>
              </div>
            </li>
          </ul>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <CardTitle>3. 生成配置</CardTitle>
        <CardDescription>
          解析完成后可微调实体、模块和资源配置，再输出 `model / payload / resource` 三个 Go 文件。
        </CardDescription>
      </CardHeader>
      <CardContent class="grid gap-6 lg:grid-cols-2">
        <div class="space-y-2">
          <Label for="entity-name">实体名称</Label>
          <Input id="entity-name" v-model="config.entityName" placeholder="User" />
        </div>

        <div class="space-y-2">
          <Label for="module-path">模块路径</Label>
          <Input id="module-path" v-model="config.modulePath" placeholder="sys 或 hr/org" />
        </div>

        <div class="space-y-2">
          <Label for="table-name">表名</Label>
          <Input id="table-name" v-model="config.tableName" placeholder="sys_user" />
        </div>

        <div class="space-y-2">
          <Label for="table-alias">表别名</Label>
          <Input id="table-alias" v-model="config.tableAlias" placeholder="u" />
        </div>

        <div class="space-y-2 lg:col-span-2">
          <Label for="rpc-path">RPC 路径</Label>
          <Input id="rpc-path" v-model="config.rpcPath" placeholder="smp/sys/user" />
        </div>

        <div class="space-y-2">
          <Label for="go-module-prefix">Go 模块前缀</Label>
          <Input id="go-module-prefix" v-model="config.goModulePrefix" placeholder="smp-server/internal" />
        </div>

        <div class="space-y-2">
          <Label for="audit-type">审计类型</Label>
          <NativeSelect id="audit-type" v-model="config.auditType" class="w-full">
            <NativeSelectOption value="fullAudited">
              FullAudited
            </NativeSelectOption>
            <NativeSelectOption value="fullTracked">
              FullTracked
            </NativeSelectOption>
            <NativeSelectOption value="creationAudited">
              CreationAudited
            </NativeSelectOption>
            <NativeSelectOption value="creationTracked">
              CreationTracked
            </NativeSelectOption>
            <NativeSelectOption value="none">
              None
            </NativeSelectOption>
          </NativeSelect>
        </div>

        <div class="space-y-2 lg:col-span-2">
          <Label for="output-dir">输出目录</Label>
          <div class="flex flex-col gap-2 lg:flex-row">
            <Input
              id="output-dir"
              v-model="config.outputDir"
              placeholder="/path/to/smp-server/internal/sys"
              class="flex-1"
            />
            <Button variant="outline" class="lg:w-auto" @click="selectOutputDir">
              <FolderOpen class="mr-2 h-4 w-4" />
              选择目录
            </Button>
          </div>
        </div>

        <div class="grid gap-3 lg:col-span-2 lg:grid-cols-2">
          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="find-page-switch">FindPage</Label>
              <p class="text-xs text-muted-foreground">
                生成分页查询资源。
              </p>
            </div>
            <Switch id="find-page-switch" v-model="config.enableFindPage" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="create-switch">Create</Label>
              <p class="text-xs text-muted-foreground">
                生成创建资源。
              </p>
            </div>
            <Switch id="create-switch" v-model="config.enableCreate" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="update-switch">Update</Label>
              <p class="text-xs text-muted-foreground">
                生成更新资源。
              </p>
            </div>
            <Switch id="update-switch" v-model="config.enableUpdate" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="delete-switch">Delete</Label>
              <p class="text-xs text-muted-foreground">
                生成删除资源。
              </p>
            </div>
            <Switch id="delete-switch" v-model="config.enableDelete" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="delete-many-switch">DeleteMany</Label>
              <p class="text-xs text-muted-foreground">
                生成批量删除资源。
              </p>
            </div>
            <Switch id="delete-many-switch" v-model="config.enableDeleteMany" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="sort-switch">默认排序</Label>
              <p class="text-xs text-muted-foreground">
                依赖 schema 中的 `SortOrder()`。
              </p>
            </div>
            <Switch id="sort-switch" v-model="config.enableSort" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="audit-user-switch">AuditUserNames</Label>
              <p class="text-xs text-muted-foreground">
                生成 `WithAuditUserNames(model.UserModel)`。
              </p>
            </div>
            <Switch id="audit-user-switch" v-model="config.enableAuditUserNames" />
          </div>

          <div class="flex items-center justify-between rounded-lg border px-4 py-3">
            <div class="space-y-1">
              <Label for="overwrite-switch">覆盖已有文件</Label>
              <p class="text-xs text-muted-foreground">
                关闭时若目标文件已存在会阻止生成。
              </p>
            </div>
            <Switch id="overwrite-switch" v-model="config.overwrite" />
          </div>
        </div>
      </CardContent>
      <CardFooter class="justify-end">
        <Button :disabled="!canGenerate" @click="handleGenerate">
          {{ generating ? '生成中...' : '生成 Go CRUD' }}
        </Button>
      </CardFooter>
    </Card>

    <Alert v-if="error" variant="destructive">
      <AlertTitle>处理失败</AlertTitle>
      <AlertDescription>{{ error }}</AlertDescription>
    </Alert>

    <Card v-if="generatedFiles.length > 0">
      <CardHeader>
        <CardTitle>4. 生成结果</CardTitle>
        <CardDescription>
          共生成 {{ generatedFiles.length }} 个文件，`module.go` 仍需你手动注册。
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <ul class="space-y-2 rounded-lg border bg-muted/30 p-4 font-mono text-sm">
          <li v-for="file of generatedFiles" :key="file" class="break-all">
            {{ file }}
          </li>
        </ul>

        <div class="rounded-lg border bg-muted/30 p-4">
          <div class="mb-3 flex items-center justify-between gap-3">
            <div>
              <p class="font-medium">
                手动注册到 `module.go`
              </p>
              <p class="text-sm text-muted-foreground">
                将下面这行加入 `vef.Module(...)` 参数列表中。
              </p>
            </div>
            <Button variant="outline" size="sm" @click="handleCopySnippet">
              <Copy class="mr-2 h-4 w-4" />
              复制
            </Button>
          </div>
          <pre class="overflow-x-auto rounded-md border bg-background px-3 py-2 text-sm"><code>{{ moduleRegistrationSnippet }}</code></pre>
        </div>
      </CardContent>
    </Card>
  </div>
</template>

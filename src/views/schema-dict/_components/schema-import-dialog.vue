<script setup lang="ts">
import { ClipboardCopy, FileUp, Loader2, Sparkles } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import type { ParsePreviewState } from '@/stores/schema-dict';
import type { SchemaDatabase, SchemaDbType, SchemaSourceKind } from '@/types/schema-dict';
import { getErrorMessage } from '@/utils/helpers';

import { schemaSummary } from '../utils/display';
import { SCHEMA_DB_TYPE_LABELS, SCHEMA_QUERY_TEMPLATES } from '../utils/query-templates';

const props = defineProps<{
  open: boolean;
  databases: SchemaDatabase[];
  activeDatabaseId: string;
  preview: ParsePreviewState | null;
  importing: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void;
  (e: 'preview', payload: { dbType: SchemaDbType; sourceKind: SchemaSourceKind; sourceText: string }): void;
  (
    e: 'confirm',
    payload: { databaseId: string; label: string; note: string; sourceKind: SchemaSourceKind; sourceText: string },
  ): void;
}>();

const databaseId = ref('');
const label = ref('');
const note = ref('');
const sourceKind = ref<SchemaSourceKind>('ddl');
const ddlText = ref('');
const queryText = ref('');
const templateOpen = ref(false);
const sqlFileInput = ref<HTMLInputElement | null>(null);
const sqlFileName = ref('');

const activeDatabase = computed(() => props.databases.find(item => item.id === databaseId.value) ?? null);
const templateText = computed(() =>
  activeDatabase.value ? (SCHEMA_QUERY_TEMPLATES[activeDatabase.value.db_type] ?? '') : '',
);
const canPreview = computed(() => currentText().trim().length > 0);
const canImport = computed(() => props.preview?.status === 'ready' && label.value.trim().length > 0);

function currentText(): string {
  return sourceKind.value === 'ddl' ? ddlText.value : queryText.value;
}

watch(
  () => props.open,
  open => {
    if (!open) {
      return;
    }
    databaseId.value = props.activeDatabaseId || props.databases[0]?.id || '';
    label.value = defaultLabel();
    note.value = '';
    sourceKind.value = 'ddl';
    ddlText.value = '';
    queryText.value = '';
    sqlFileName.value = '';
    if (sqlFileInput.value) {
      sqlFileInput.value.value = '';
    }
    templateOpen.value = false;
  },
  { immediate: true },
);

function defaultLabel(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

async function handleSqlFileChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }
  try {
    ddlText.value = await file.text();
    sqlFileName.value = file.name;
    toast.success(`已读取 ${file.name}`);
  } catch (error) {
    toast.error(getErrorMessage(error, '读取 SQL 文件失败'));
  }
}

function requestPreview() {
  emit('preview', {
    dbType: activeDatabase.value?.db_type ?? 'mysql',
    sourceKind: sourceKind.value,
    sourceText: currentText(),
  });
}

function confirmImport() {
  if (!canImport.value) {
    return;
  }
  emit('confirm', {
    databaseId: databaseId.value,
    label: label.value.trim(),
    note: note.value,
    sourceKind: sourceKind.value,
    sourceText: currentText(),
  });
}

function setDbType(value: unknown) {
  // 库条目的方言由库里配置，选择器只切换"当前编辑的库"
  databaseId.value = value as string;
}

async function copyTemplate() {
  try {
    await navigator.clipboard.writeText(templateText.value);
    toast.success('查询语句已复制，去 SQL 客户端执行后把结果粘贴回来');
  } catch {
    toast.error('复制失败，请手动选择文本复制');
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="value => emit('update:open', value)">
    <DialogContent class="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>导入表结构快照</DialogTitle>
        <DialogDescription>
          粘贴建表 DDL，或在 SQL 客户端里跑一条查询把结果粘回来；保存后自动和上一版对比生成变更时间线。
        </DialogDescription>
      </DialogHeader>

      <div class="grid gap-4">
        <div class="grid gap-4 sm:grid-cols-2">
          <div class="space-y-1.5">
            <Label>目标库</Label>
            <Select :model-value="databaseId" @update:model-value="setDbType">
              <SelectTrigger aria-label="目标库">
                <SelectValue placeholder="选择库" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item of databases" :key="item.id" :value="item.id">
                  {{ item.name }}（{{ SCHEMA_DB_TYPE_LABELS[item.db_type] }}）
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label for="schema-import-label">版本名</Label>
            <Input id="schema-import-label" v-model="label" placeholder="如：2026-10 升级前" />
          </div>
        </div>

        <Tabs v-model="sourceKind" class="gap-3">
          <TabsList>
            <TabsTrigger value="ddl">贴 DDL</TabsTrigger>
            <TabsTrigger value="query">贴查询结果</TabsTrigger>
          </TabsList>

          <TabsContent value="ddl" class="space-y-1.5">
            <div class="flex items-center justify-between gap-2">
              <Label for="schema-import-ddl">建表 DDL（CREATE TABLE 语句）</Label>
              <Button type="button" variant="outline" size="sm" @click="sqlFileInput?.click()">
                <FileUp class="size-3.5" />
                读取 SQL 文件
              </Button>
            </div>
            <p v-if="sqlFileName" class="text-xs text-muted-foreground">已载入文件：{{ sqlFileName }}</p>
            <Textarea
              id="schema-import-ddl"
              v-model="ddlText"
              rows="10"
              class="font-mono text-xs"
              placeholder="粘贴建表语句，或点右上角读取 .sql 文件；支持 mysqldump / pg_dump / Oracle 导出，SET、LOCK 等无关语句会被自动跳过"
            />
            <input
              ref="sqlFileInput"
              type="file"
              accept=".sql,.ddl,.txt"
              class="hidden"
              @change="handleSqlFileChange"
            />
          </TabsContent>

          <TabsContent value="query" class="space-y-1.5">
            <div class="flex items-center justify-between gap-2">
              <Label for="schema-import-query">查询结果（JSON 或带表头的 TSV）</Label>
              <Button
                type="button"
                variant="outline"
                size="sm"
                :disabled="!activeDatabase"
                @click="templateOpen = true"
              >
                <ClipboardCopy class="size-3.5" />
                查询模板
              </Button>
            </div>
            <Textarea
              id="schema-import-query"
              v-model="queryText"
              rows="10"
              class="font-mono text-xs"
              placeholder="在 SQL 客户端执行「查询模板」里的语句，复制结果集（含表头）粘贴到这里"
            />
          </TabsContent>
        </Tabs>

        <!-- 解析预览 -->
        <div
          v-if="preview"
          class="flex items-start gap-2 rounded-lg border px-3 py-2.5 text-sm"
          :class="
            preview.status === 'error' ? 'border-destructive/30 bg-destructive/5' : 'border-primary/30 bg-primary/5'
          "
        >
          <Loader2 v-if="preview.status === 'parsing'" class="mt-0.5 size-4 animate-spin shrink-0" />
          <Sparkles v-else-if="preview.status === 'ready'" class="mt-0.5 size-4 shrink-0 text-primary" />
          <div class="min-w-0 text-xs leading-5" :class="preview.status === 'error' ? 'text-destructive' : ''">
            <template v-if="preview.status === 'parsing'">正在解析…</template>
            <template v-else-if="preview.status === 'error'">{{ preview.errorMessage }}</template>
            <template v-else>
              解析成功：{{ schemaSummary(preview.schema!) }}
              <span
                v-if="preview.schema && preview.schema.tables.length > 0"
                class="mt-1 block truncate text-muted-foreground"
              >
                {{
                  preview.schema.tables
                    .slice(0, 6)
                    .map(t => t.name)
                    .join('、')
                }}{{ preview.schema.tables.length > 6 ? ' …' : '' }}
              </span>
            </template>
          </div>
        </div>

        <div class="space-y-1.5">
          <Label for="schema-import-note">备注（可选）</Label>
          <Input id="schema-import-note" v-model="note" placeholder="这次导入的背景，如：供应商 10 月升级包" />
        </div>
      </div>

      <DialogFooter class="mt-2">
        <Button type="button" variant="outline" @click="emit('update:open', false)"> 取消 </Button>
        <Button
          type="button"
          variant="outline"
          :disabled="!canPreview || preview?.status === 'parsing'"
          @click="requestPreview"
        >
          解析预览
        </Button>
        <Button type="button" :disabled="!canImport || importing" @click="confirmImport">
          <Loader2 v-if="importing" class="size-4 animate-spin" />
          保存快照
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <!-- 查询模板 -->
  <Dialog :open="templateOpen" @update:open="value => (templateOpen = value)">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle
          >{{ activeDatabase ? SCHEMA_DB_TYPE_LABELS[activeDatabase.db_type] : '' }} 智能查询模板</DialogTitle
        >
        <DialogDescription>
          在你的 SQL 客户端里执行这条语句，把完整结果集（含表头）复制粘贴到导入框即可。
        </DialogDescription>
      </DialogHeader>
      <Textarea :model-value="templateText" rows="14" readonly class="font-mono text-xs" />
      <DialogFooter>
        <Button type="button" variant="outline" @click="templateOpen = false"> 关闭 </Button>
        <Button type="button" @click="copyTemplate">
          <ClipboardCopy class="size-4" />
          复制语句
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

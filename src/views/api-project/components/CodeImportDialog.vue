<script setup lang="ts">
import { open as openDirectoryDialog } from '@tauri-apps/plugin-dialog';
import { ArrowRight, CheckCircle2, FolderInput, Loader2, TriangleAlert } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { toast } from 'vue-sonner';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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
import { CODE_IMPORT_FRAMEWORKS, findCodeImportFramework } from '@/services/api-client/code-import/frameworks';
import {
  cancelCodeExport,
  exportCodeManifest,
  listApiImportSources,
  runApiImport,
} from '@/services/api-client/code-import/service';
import type { CodeImportPreview, TauriImportSummary } from '@/services/api-client/code-import/types';
import { listApiEnvironments, upsertApiEnvironment } from '@/services/api-client/environments';
import type { ApiClientProject } from '@/types/api-client';

import CodeImportTree from './CodeImportTree.vue';

interface Props {
  open: boolean;
  project: ApiClientProject | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
}>();

const router = useRouter();

type Step = 'source' | 'preview' | 'options' | 'result';

const step = ref<Step>('source');
const frameworkId = ref<string>(CODE_IMPORT_FRAMEWORKS[0]?.id ?? '');
const repoPath = ref('');
const baseUrl = ref('');
const replaceExisting = ref(true);
const exporting = ref(false);
const importing = ref(false);
const exportError = ref<string | null>(null);
const importError = ref<string | null>(null);
const cancelledNotice = ref(false);
const preview = ref<CodeImportPreview | null>(null);
const selectedKeys = ref<Set<string>>(new Set());
const summary = ref<TauriImportSummary | null>(null);
let exportId = '';

const framework = computed(() => findCodeImportFramework(frameworkId.value));
const sourceReady = computed(() => Boolean(framework.value) && repoPath.value.trim().length > 0);

const selectedRequestCount = computed(() => {
  if (!preview.value) {
    return 0;
  }
  let count = 0;
  for (const module of preview.value.modules) {
    for (const resource of module.resources) {
      if (selectedKeys.value.has(resource.key)) {
        count += resource.operations.length;
      }
    }
  }
  return count;
});

const totalResourceCount = computed(
  () => preview.value?.modules.reduce((sum, module) => sum + module.resources.length, 0) ?? 0,
);

const steps: { key: Step; label: string }[] = [
  { key: 'source', label: '选择来源' },
  { key: 'preview', label: '预览接口' },
  { key: 'options', label: '导入设置' },
];

function reset(): void {
  step.value = 'source';
  exporting.value = false;
  importing.value = false;
  exportError.value = null;
  importError.value = null;
  cancelledNotice.value = false;
  preview.value = null;
  selectedKeys.value = new Set();
  summary.value = null;
  replaceExisting.value = true;
  exportId = '';
}

watch(
  () => props.open,
  async opened => {
    if (!opened || !props.project) {
      return;
    }
    reset();
    try {
      const sources = await listApiImportSources(props.project.id);
      const saved = sources[0];
      if (saved) {
        frameworkId.value = saved.framework;
        repoPath.value = saved.repo_path;
      }
    } catch {
      // 导入源是回填项，读取失败不阻断向导
    }
    baseUrl.value = framework.value?.defaultBaseUrl ?? '';
  },
);

function onOpenChange(open: boolean): void {
  if (!open && (exporting.value || importing.value)) {
    void cancelCodeExport(exportId);
  }
  emit('update:open', open);
}

async function chooseRepoDirectory(): Promise<void> {
  const selected = await openDirectoryDialog({ directory: true, title: '选择后端代码仓库' });
  if (typeof selected === 'string') {
    repoPath.value = selected;
  }
}

async function startExport(): Promise<void> {
  if (!props.project || !framework.value) {
    return;
  }
  exporting.value = true;
  exportError.value = null;
  cancelledNotice.value = false;
  exportId = `exp_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

  try {
    const json = await exportCodeManifest({
      framework: framework.value.id,
      repoPath: repoPath.value.trim(),
      exportId,
    });
    preview.value = framework.value.parseManifest(json);
    selectedKeys.value = new Set(
      preview.value.modules.flatMap(module => module.resources.map(resource => resource.key)),
    );
    step.value = 'preview';
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes('已取消')) {
      cancelledNotice.value = true;
    } else {
      exportError.value = message;
    }
  } finally {
    exporting.value = false;
  }
}

async function cancelExport(): Promise<void> {
  try {
    await cancelCodeExport(exportId);
  } catch {
    // 后台任务会随下一次完成/失败自然结束
  }
}

async function startImport(): Promise<void> {
  if (!props.project || !framework.value || !preview.value) {
    return;
  }
  importing.value = true;
  importError.value = null;

  try {
    const payload = framework.value.buildImportPayload(preview.value, selectedKeys.value);
    summary.value = await runApiImport({
      projectId: props.project.id,
      framework: framework.value.id,
      repoPath: repoPath.value.trim(),
      replaceExisting: replaceExisting.value,
      payload,
    });
    await ensureDevEnvironment(props.project.id, baseUrl.value.trim());
    step.value = 'result';
    toast.success(`已导入 ${summary.value.created_requests} 个接口`);
  } catch (error) {
    importError.value = error instanceof Error ? error.message : String(error);
  } finally {
    importing.value = false;
  }
}

/** 创建或更新「开发」环境：写入 baseUrl，并保证 token 变量存在 */
async function ensureDevEnvironment(projectId: string, url: string): Promise<void> {
  if (!url) {
    return;
  }
  const environments = await listApiEnvironments(projectId);
  const dev = environments.find(environment => environment.name === '开发');
  const variables = dev?.variables ?? [];
  const nextVariables = variables.some(variable => variable.key === 'token')
    ? variables
    : [...variables, { id: `var_${Date.now()}`, enabled: true, key: 'token', value: '' }];

  await upsertApiEnvironment({
    id: dev?.id ?? '',
    projectId,
    name: '开发',
    baseUrl: url,
    variables: nextVariables,
    createdAt: dev?.createdAt ?? 0,
    updatedAt: dev?.updatedAt ?? 0,
  });
}

function openWorkspace(): void {
  if (!props.project) {
    return;
  }
  emit('update:open', false);
  void router.push({ name: 'api-client-project', params: { id: props.project.id } });
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="flex h-[640px] max-h-[88vh] w-full max-w-2xl flex-col gap-0 p-0 sm:max-w-2xl">
      <DialogHeader class="border-b px-6 py-4">
        <DialogTitle>从代码导入 · {{ project?.name }}</DialogTitle>
        <DialogDescription>
          <nav v-if="step !== 'result'" class="flex items-center gap-2 pt-1 text-xs" aria-label="导入步骤">
            <template v-for="(item, index) of steps" :key="item.key">
              <span v-if="index > 0" class="text-muted-foreground/40" aria-hidden="true">—</span>
              <span
                class="rounded-full px-2 py-0.5 transition-colors duration-150"
                :class="
                  step === item.key
                    ? 'bg-primary/10 font-medium text-primary'
                    : steps.findIndex(s => s.key === step) > index
                      ? 'text-muted-foreground'
                      : 'text-muted-foreground/50'
                "
              >
                {{ index + 1 }}. {{ item.label }}
              </span>
            </template>
          </nav>
          <span v-else class="pt-1">导入完成</span>
        </DialogDescription>
      </DialogHeader>

      <!-- 第一步：选择来源 -->
      <div v-if="step === 'source'" class="flex flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
        <div class="flex flex-col gap-2">
          <Label>后端框架</Label>
          <Select v-model="frameworkId">
            <SelectTrigger class="w-full">
              <SelectValue placeholder="选择框架" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="item of CODE_IMPORT_FRAMEWORKS" :key="item.id" :value="item.id">
                {{ item.label }}
              </SelectItem>
            </SelectContent>
          </Select>
          <p class="text-xs text-muted-foreground">{{ framework?.description }}</p>
        </div>

        <div class="flex flex-col gap-2">
          <Label>代码仓库路径</Label>
          <div class="flex gap-2">
            <Input
              v-model="repoPath"
              placeholder="/Users/you/project/smp-server"
              class="font-mono text-sm"
              @keydown.enter="sourceReady && !exporting && startExport()"
            />
            <Button variant="outline" @click="chooseRepoDirectory">
              <FolderInput class="size-4" />
              选择目录
            </Button>
          </div>
          <p class="text-xs text-muted-foreground">
            将在仓库目录执行框架自带的清单导出命令。首次运行需要编译，可能需要 1–2 分钟。
          </p>
        </div>

        <div v-if="exporting" class="flex flex-col gap-3 rounded-md border bg-muted/30 px-4 py-3">
          <div class="flex items-center gap-2 text-sm">
            <Loader2 class="size-4 animate-spin text-primary" />
            正在读取接口清单…
          </div>
          <p class="text-xs text-muted-foreground">冷缓存下 go run 需要编译依赖，请耐心等待。</p>
          <Button variant="outline" size="sm" class="self-start" @click="cancelExport">取消</Button>
        </div>

        <Alert v-else-if="exportError" variant="destructive">
          <TriangleAlert class="size-4" />
          <AlertTitle>读取接口失败</AlertTitle>
          <AlertDescription class="whitespace-pre-wrap break-all">{{ exportError }}</AlertDescription>
        </Alert>

        <p v-else-if="cancelledNotice" class="text-sm text-muted-foreground">已取消读取，可重新开始。</p>
      </div>

      <!-- 第二步：预览接口 -->
      <div v-else-if="step === 'preview' && preview" class="flex min-h-0 flex-1 flex-col gap-4 px-6 py-5">
        <div class="flex items-baseline gap-6">
          <p>
            <span class="text-2xl font-semibold tabular-nums text-foreground">
              {{ preview.modules.length }}
            </span>
            <span class="ml-1 text-sm text-muted-foreground">个模块</span>
          </p>
          <p>
            <span class="text-2xl font-semibold tabular-nums text-foreground">
              {{ totalResourceCount }}
            </span>
            <span class="ml-1 text-sm text-muted-foreground">个资源</span>
          </p>
          <p>
            <span class="text-2xl font-semibold tabular-nums text-foreground">
              {{ preview.totalRequests.toLocaleString() }}
            </span>
            <span class="ml-1 text-sm text-muted-foreground">个接口</span>
          </p>
        </div>

        <CodeImportTree
          :modules="preview.modules"
          :selected-keys="selectedKeys"
          @update:selected-keys="value => (selectedKeys = value)"
        />
      </div>

      <!-- 第三步：导入设置 -->
      <div v-else-if="step === 'options'" class="flex flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
        <div class="flex flex-col gap-2">
          <Label>环境地址（base_url）</Label>
          <Input v-model="baseUrl" placeholder="http://127.0.0.1:8100" class="font-mono text-sm" />
          <p class="text-xs text-muted-foreground">
            导入后会创建或更新「开发」环境并写入该地址，同时补充
            <code class="font-mono">token</code> 变量。请求地址为相对路径
            <code class="font-mono">/api</code>，执行时按所选环境拼接。
          </p>
        </div>

        <label class="flex cursor-pointer items-start gap-3 rounded-md border px-4 py-3">
          <input
            type="checkbox"
            class="mt-0.5 size-4 shrink-0 accent-primary"
            :checked="replaceExisting"
            @change="replaceExisting = ($event.target as HTMLInputElement).checked"
          />
          <span class="flex flex-col gap-1">
            <span class="text-sm font-medium text-foreground">清空现有分组后重建</span>
            <span class="text-xs text-muted-foreground">
              删除本项目当前所有分组及其请求（含执行历史），再按清单建立「模块 &gt;
              资源」两级分组。关闭此选项则保留现有分组，在后面追加。
            </span>
          </span>
        </label>

        <div class="rounded-md border bg-muted/30 px-4 py-3 text-sm">
          <p class="font-medium text-foreground">即将导入</p>
          <p class="mt-1 text-xs tabular-nums text-muted-foreground">
            {{ selectedKeys.size }} 个资源 · {{ selectedRequestCount }} 个接口
            <template v-if="replaceExisting">，清空现有分组</template>
          </p>
        </div>

        <Alert v-if="importError" variant="destructive">
          <TriangleAlert class="size-4" />
          <AlertTitle>导入失败</AlertTitle>
          <AlertDescription class="whitespace-pre-wrap break-all">{{ importError }}</AlertDescription>
        </Alert>
      </div>

      <!-- 结果 -->
      <div v-else-if="step === 'result' && summary" class="flex flex-1 flex-col items-center gap-4 px-6 py-10">
        <CheckCircle2 class="size-10 text-primary" />
        <div class="text-center">
          <p class="text-lg font-semibold text-foreground">导入完成</p>
          <p class="mt-1 text-sm tabular-nums text-muted-foreground">
            新建 {{ summary.created_groups }} 个分组、{{ summary.created_requests }} 个接口<template
              v-if="summary.deleted_groups > 0"
              >；已清空 {{ summary.deleted_groups }} 个旧分组、{{ summary.deleted_requests }} 个旧请求</template
            >
          </p>
          <p class="mt-3 text-xs text-muted-foreground">
            先在工作区执行「auth · login」获取 token，粘贴到开发环境的 token 变量后即可调用其他接口。
          </p>
        </div>
        <Button @click="openWorkspace">
          进入工作区
          <ArrowRight class="size-4" />
        </Button>
      </div>

      <DialogFooter class="border-t px-6 py-4">
        <template v-if="step === 'source'">
          <Button variant="outline" @click="emit('update:open', false)">取消</Button>
          <Button :disabled="!sourceReady || exporting" @click="startExport">
            <Loader2 v-if="exporting" class="size-4 animate-spin" />
            {{ exporting ? '读取中…' : '读取接口' }}
          </Button>
        </template>
        <template v-else-if="step === 'preview'">
          <Button variant="outline" @click="step = 'source'">上一步</Button>
          <Button :disabled="selectedKeys.size === 0" @click="step = 'options'">
            下一步
            <ArrowRight class="size-4" />
          </Button>
        </template>
        <template v-else-if="step === 'options'">
          <Button variant="outline" :disabled="importing" @click="step = 'preview'">上一步</Button>
          <Button :disabled="selectedKeys.size === 0 || importing" @click="startImport">
            <Loader2 v-if="importing" class="size-4 animate-spin" />
            {{ importing ? '导入中…' : `开始导入（${selectedRequestCount}）` }}
          </Button>
        </template>
        <template v-else>
          <Button variant="outline" @click="emit('update:open', false)">关闭</Button>
        </template>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

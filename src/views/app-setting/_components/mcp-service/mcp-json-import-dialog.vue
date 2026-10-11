<script setup lang="ts">
import { ClipboardPaste, Loader2, TriangleAlert } from 'lucide-vue-next';
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { upsertMcpService } from '@/services';
import type { McpService } from '@/types/mcp-service';
import { getErrorMessage } from '@/utils/helpers';

import { argsToLines, envToLines, parseMcpServersJson } from './parse-mcp-json';
import type { ParsedMcpServerConfig } from './parse-mcp-json';

const props = defineProps<{
  open: boolean;
  /** 已有服务，用于同名冲突检测 */
  services: McpService[];
}>();

const emit = defineEmits<{
  (e: 'update:open', open: boolean): void;
  (e: 'imported'): void;
}>();

const rawText = ref('');
const parsed = ref<ParsedMcpServerConfig[] | null>(null);
const parseErrors = ref<string[]>([]);
const selected = ref<Record<string, boolean>>({});
const importing = ref(false);

const existingNames = computed(() => new Set(props.services.map(service => service.name)));

/** 同名冲突的默认不勾选 */
function nameExists(name: string): boolean {
  return existingNames.value.has(name);
}

const checkedCount = computed(() => parsed.value?.filter(config => selected.value[config.name]).length ?? 0);

watch(
  () => props.open,
  open => {
    if (!open) {
      return;
    }
    rawText.value = '';
    parsed.value = null;
    parseErrors.value = [];
    selected.value = {};
    importing.value = false;
  },
);

function runParse() {
  const result = parseMcpServersJson(rawText.value);
  parsed.value = result.services;
  parseErrors.value = result.errors;
  const nextSelected: Record<string, boolean> = {};
  for (const config of result.services) {
    nextSelected[config.name] = !nameExists(config.name);
  }
  selected.value = nextSelected;
}

function resetSelection() {
  for (const config of parsed.value ?? []) {
    selected.value[config.name] = !nameExists(config.name);
  }
}

async function pasteFromClipboard() {
  try {
    rawText.value = await navigator.clipboard.readText();
    toast.success('已从剪贴板粘贴');
  } catch {
    toast.error('读取剪贴板失败，请手动粘贴');
  }
}

function toPayload(config: ParsedMcpServerConfig) {
  return {
    id: '',
    name: config.name,
    description: '',
    enabled: config.enabled,
    transport_type: config.transportType,
    command: config.command,
    args: argsToLines(config.args),
    env: envToLines(config.env),
    url: config.url,
    long_running: false,
    timeout_seconds: 60,
    discovery: null,
  };
}

async function importSelected() {
  const targets = (parsed.value ?? []).filter(config => selected.value[config.name]);
  if (targets.length === 0) {
    return;
  }

  importing.value = true;
  try {
    let imported = 0;
    for (const config of targets) {
      await upsertMcpService(toPayload(config));
      imported += 1;
    }
    toast.success(`已导入 ${imported} 个 MCP 服务`);
    emit('update:open', false);
    emit('imported');
  } catch (error) {
    toast.error(getErrorMessage(error, '导入 MCP 服务失败'));
  } finally {
    importing.value = false;
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="value => emit('update:open', value)">
    <DialogContent class="max-h-[85vh] overflow-y-auto sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>从 JSON 导入 MCP 服务</DialogTitle>
        <DialogDescription>
          粘贴 Claude Desktop / Cursor 格式的 mcpServers 配置，一次导入多个服务；同名服务会被跳过。
        </DialogDescription>
      </DialogHeader>

      <div class="grid gap-4">
        <div class="space-y-1.5">
          <div class="flex items-center justify-between gap-2">
            <Label for="mcp-json-import">JSON 配置</Label>
            <Button type="button" variant="outline" size="sm" @click="pasteFromClipboard">
              <ClipboardPaste class="size-3.5" />
              粘贴
            </Button>
          </div>
          <Textarea
            id="mcp-json-import"
            v-model="rawText"
            rows="9"
            class="font-mono text-xs"
            placeholder='{
  "mcpServers": {
    "fetch": { "command": "uvx", "args": ["mcp-server-fetch"] },
    "eidolon": { "url": "http://127.0.0.1:9090/mcp" }
  }
}'
          />
        </div>

        <!-- 解析错误 -->
        <div
          v-if="parseErrors.length > 0"
          class="space-y-1.5 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2.5"
        >
          <p class="flex items-center gap-1.5 text-sm font-medium text-destructive">
            <TriangleAlert class="size-4" />
            {{ parseErrors[0] }}
          </p>
          <p v-for="error of parseErrors.slice(1)" :key="error" class="text-xs text-destructive/90">
            {{ error }}
          </p>
        </div>

        <!-- 解析预览 -->
        <div v-if="parsed && parsed.length > 0" class="space-y-2">
          <div class="flex items-center justify-between">
            <Label>解析出 {{ parsed.length }} 个服务</Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              class="h-7 text-xs text-muted-foreground"
              @click="resetSelection"
            >
              重置选择
            </Button>
          </div>

          <div class="space-y-1.5">
            <label
              v-for="config of parsed"
              :key="config.name"
              class="flex cursor-pointer items-start gap-2.5 rounded-lg border px-3 py-2.5 transition-colors"
              :class="selected[config.name] ? 'border-primary/50 bg-primary/5' : 'hover:bg-muted/40'"
              :aria-disabled="nameExists(config.name)"
            >
              <input
                v-model="selected[config.name]"
                type="checkbox"
                class="mt-0.5 size-4 accent-primary"
                :disabled="nameExists(config.name)"
              />
              <span class="min-w-0 flex-1">
                <span class="flex flex-wrap items-center gap-2">
                  <span class="truncate text-sm font-medium">{{ config.name }}</span>
                  <Badge variant="outline" class="border-primary/20 bg-primary/10 text-primary">
                    {{ config.transportType === 'stdio' ? 'STDIO' : 'HTTP' }}
                  </Badge>
                  <Badge
                    v-if="nameExists(config.name)"
                    variant="outline"
                    class="border-border bg-muted text-muted-foreground"
                  >
                    同名已存在，将跳过
                  </Badge>
                  <Badge v-if="!config.enabled" variant="outline" class="border-border bg-muted text-muted-foreground">
                    导入为禁用
                  </Badge>
                </span>
                <span class="mt-0.5 block truncate font-mono text-xs text-muted-foreground">
                  {{ config.transportType === 'stdio' ? [config.command, ...config.args].join(' ') : config.url }}
                </span>
              </span>
            </label>
          </div>
        </div>
      </div>

      <DialogFooter class="mt-2">
        <Button type="button" variant="outline" @click="emit('update:open', false)"> 取消 </Button>
        <Button type="button" variant="outline" :disabled="!rawText.trim() || importing" @click="runParse">
          解析预览
        </Button>
        <Button type="button" :disabled="checkedCount === 0 || importing" @click="importSelected">
          <Loader2 v-if="importing" class="size-4 animate-spin" />
          导入 {{ checkedCount > 0 ? `（${checkedCount}）` : '' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

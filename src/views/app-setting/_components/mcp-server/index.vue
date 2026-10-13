<script setup lang="ts">
import { Check, Copy, PlugZap, RefreshCw } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  getMcpServerInfo,
  regenerateMcpServerToken,
  updateMcpServerSettings,
  type McpServerInfo,
} from '@/services/mcp_server';
import { getErrorMessage } from '@/utils/helpers';

const info = ref<McpServerInfo | null>(null);
const loading = ref(false);
const saving = ref(false);
const regenerating = ref(false);
const portDraft = ref('');
const copied = ref<'config' | 'token' | null>(null);

const endpointUrl = computed(() => {
  if (!info.value?.running) {
    return '';
  }
  return `http://127.0.0.1:${info.value.activePort ?? info.value.port}/mcp`;
});

const portChanged = computed(() => {
  if (!info.value) {
    return false;
  }
  const parsed = Number.parseInt(portDraft.value, 10);
  return Number.isFinite(parsed) && parsed > 0 && parsed <= 65_535 && parsed !== info.value.port;
});

const clientConfig = computed(() => {
  if (!endpointUrl.value || !info.value) {
    return '';
  }
  return JSON.stringify(
    {
      mcpServers: {
        eidolon: {
          type: 'http',
          url: endpointUrl.value,
          headers: { Authorization: `Bearer ${info.value.token}` },
        },
      },
    },
    null,
    2,
  );
});

async function load(): Promise<void> {
  loading.value = true;
  try {
    info.value = await getMcpServerInfo();
    portDraft.value = String(info.value.port);
  } catch (error) {
    toast.error(getErrorMessage(error, '加载 MCP 服务信息失败'));
  } finally {
    loading.value = false;
  }
}

async function handleToggle(enabled: boolean): Promise<void> {
  if (!info.value) {
    return;
  }
  saving.value = true;
  try {
    info.value = await updateMcpServerSettings({ enabled });
    portDraft.value = String(info.value.port);
    toast.success(enabled ? 'MCP 服务已开启' : 'MCP 服务已关闭');
  } catch (error) {
    toast.error(getErrorMessage(error, '更新 MCP 服务失败'));
    await load();
  } finally {
    saving.value = false;
  }
}

async function handleSavePort(): Promise<void> {
  if (!portChanged.value) {
    return;
  }
  saving.value = true;
  try {
    info.value = await updateMcpServerSettings({ port: Number.parseInt(portDraft.value, 10) });
    toast.success('端口已应用');
  } catch (error) {
    toast.error(getErrorMessage(error, '应用端口失败'));
    await load();
  } finally {
    saving.value = false;
  }
}

async function handleRegenerateToken(): Promise<void> {
  regenerating.value = true;
  try {
    info.value = await regenerateMcpServerToken();
    toast.success('访问令牌已重新生成，使用旧令牌的客户端需要更新配置');
  } catch (error) {
    toast.error(getErrorMessage(error, '重新生成令牌失败'));
  } finally {
    regenerating.value = false;
  }
}

async function copyText(text: string, kind: 'config' | 'token'): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = kind;
    setTimeout(() => (copied.value = null), 1500);
  } catch {
    toast.error('复制失败，请手动选择文本复制');
  }
}

onMounted(() => {
  void load();
});
</script>

<template>
  <div class="min-h-0 flex-1 overflow-y-auto rounded-xl border bg-card">
    <div class="mx-auto flex max-w-2xl flex-col gap-6 p-6">
      <div class="flex items-start justify-between gap-4">
        <div class="flex flex-col gap-1">
          <h2 class="flex items-center gap-2 text-sm font-semibold">
            <PlugZap class="h-4 w-4 text-primary" />
            对外 MCP 服务
          </h2>
          <p class="text-xs leading-5 text-muted-foreground">
            把文档库和个人任务以 MCP 工具的形式提供给本机其他程序（Claude Code、Cursor 等），AI
            可以直接读写你的文档、记录和跟进任务（任务不提供删除）。服务只监听本机，文档保存在原有磁盘目录，任务存入应用数据库。
          </p>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <Badge v-if="info?.running" class="bg-primary/10 text-primary" variant="outline">
            运行中 · {{ info.activePort ?? info.port }}
          </Badge>
          <Badge v-else variant="outline" class="text-muted-foreground">已停止</Badge>
          <Switch
            :model-value="info?.enabled ?? false"
            :disabled="saving || loading"
            @update:model-value="handleToggle"
          />
        </div>
      </div>

      <template v-if="info?.running">
        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">服务地址</Label>
          <code class="rounded-md border bg-muted/40 px-3 py-2 font-mono text-xs">{{ endpointUrl }}</code>
        </div>

        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">客户端配置（粘贴到 Claude Code / ZCode 等的 MCP 设置里）</Label>
          <div class="relative">
            <pre class="overflow-x-auto rounded-md border bg-muted/40 px-3 py-2.5 pr-12 font-mono text-xs leading-5">{{
              clientConfig
            }}</pre>
            <Button
              variant="ghost"
              size="icon-sm"
              class="absolute top-1.5 right-1.5"
              aria-label="复制客户端配置"
              @click="copyText(clientConfig, 'config')"
            >
              <Check v-if="copied === 'config'" class="size-4 text-primary" />
              <Copy v-else class="size-4" />
            </Button>
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">访问令牌</Label>
          <div class="flex items-center gap-2">
            <code class="min-w-0 flex-1 truncate rounded-md border bg-muted/40 px-3 py-2 font-mono text-xs">
              {{ info.token }}
            </code>
            <Button
              variant="ghost"
              size="icon-sm"
              :aria-label="copied === 'token' ? '已复制' : '复制令牌'"
              @click="copyText(info.token, 'token')"
            >
              <Check v-if="copied === 'token'" class="size-4 text-primary" />
              <Copy v-else class="size-4" />
            </Button>
            <Button variant="outline" size="sm" :disabled="regenerating" @click="handleRegenerateToken">
              <RefreshCw class="size-3.5" :class="regenerating && 'animate-spin'" />
              重新生成
            </Button>
          </div>
          <p class="text-xs text-muted-foreground">重新生成后，已配置的客户端需要更新令牌才能继续连接。</p>
        </div>

        <div class="flex flex-col gap-2">
          <Label class="text-xs text-muted-foreground">监听端口</Label>
          <div class="flex items-center gap-2">
            <Input v-model="portDraft" class="w-32 font-mono text-sm" @keydown.enter="handleSavePort" />
            <Button v-if="portChanged" size="sm" :disabled="saving" @click="handleSavePort">应用</Button>
          </div>
          <p v-if="info.activePort !== info.port" class="text-xs text-amber-600">
            配置端口 {{ info.port }} 被占用，实际使用 {{ info.activePort }}。客户端配置请以服务地址为准。
          </p>
        </div>
      </template>

      <p v-else class="rounded-md border border-dashed px-4 py-6 text-center text-xs text-muted-foreground">
        打开右上角开关即可启用。外部程序将以
        <code class="font-mono">http://127.0.0.1:&lt;端口&gt;/mcp</code> 连接。
      </p>
    </div>
  </div>
</template>

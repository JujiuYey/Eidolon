<script setup lang="ts">
import { computed, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderOpen, Sparkles } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { generateFrontendCrud } from '@/services/codegen';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { getErrorMessage } from '@/utils/helpers';

const resourcePath = ref('');
const frontendBasePath = ref('');
const overwrite = ref(false);
const loading = ref(false);
const generatedFiles = ref<string[]>([]);
const error = ref('');

const isReady = computed(() => {
  return resourcePath.value.trim().length > 0 && frontendBasePath.value.trim().length > 0;
});

async function selectResourceFile() {
  const selected = await open({
    multiple: false,
    title: '选择 Go Resource 文件',
    filters: [{ name: 'Go Files', extensions: ['go'] }],
  });

  if (selected && typeof selected === 'string') {
    resourcePath.value = selected;
    error.value = '';
  }
}

async function selectFrontendDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择前端项目 src 目录',
  });

  if (selected && typeof selected === 'string') {
    frontendBasePath.value = selected;
    error.value = '';
  }
}

async function handleGenerate() {
  if (!isReady.value) {
    toast.error('请先选择 Resource 文件和前端目录');
    return;
  }

  loading.value = true;
  error.value = '';
  generatedFiles.value = [];

  try {
    const result = await generateFrontendCrud({
      resourcePath: resourcePath.value.trim(),
      frontendBasePath: frontendBasePath.value.trim(),
      overwrite: overwrite.value,
    });

    generatedFiles.value = result;
    toast.success(`生成完成，共输出 ${result.length} 个文件`);
  } catch (err) {
    error.value = getErrorMessage(err, '生成失败');
    toast.error(error.value);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="space-y-6">
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Sparkles class="h-4 w-4 text-primary" />
          前端 CRUD 生成
        </CardTitle>
        <CardDescription>
          从 Go 后端 Resource 文件生成前端 TypeScript API 与 React CRUD 页面。
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-6">
        <div class="space-y-2">
          <Label for="resource-path">Go Resource 文件路径</Label>
          <div class="flex flex-col gap-2 lg:flex-row">
            <Input
              id="resource-path"
              v-model="resourcePath"
              placeholder="internal/sys/resource/app.go"
              class="flex-1"
            />
            <Button variant="outline" class="lg:w-auto" @click="selectResourceFile">
              <FolderOpen class="mr-2 h-4 w-4" />
              选择文件
            </Button>
          </div>
        </div>

        <div class="space-y-2">
          <Label for="frontend-base-path">前端项目 src 目录</Label>
          <div class="flex flex-col gap-2 lg:flex-row">
            <Input
              id="frontend-base-path"
              v-model="frontendBasePath"
              placeholder="/path/to/smp-web/src"
              class="flex-1"
            />
            <Button variant="outline" class="lg:w-auto" @click="selectFrontendDir">
              <FolderOpen class="mr-2 h-4 w-4" />
              选择目录
            </Button>
          </div>
        </div>

        <div class="flex items-center justify-between rounded-lg border px-4 py-3">
          <div class="space-y-1">
            <Label for="frontend-overwrite-switch">覆盖已有文件</Label>
            <p class="text-sm text-muted-foreground">
              关闭时若检测到目标文件已存在，会直接阻止生成。
            </p>
          </div>

          <Switch
            id="frontend-overwrite-switch"
            v-model="overwrite"
            aria-label="是否覆盖已有文件"
          />
        </div>
      </CardContent>
      <CardFooter class="justify-end">
        <Button :disabled="loading || !isReady" @click="handleGenerate">
          {{ loading ? '生成中...' : '开始生成' }}
        </Button>
      </CardFooter>
    </Card>

    <Alert v-if="error" variant="destructive">
      <AlertTitle>生成失败</AlertTitle>
      <AlertDescription>{{ error }}</AlertDescription>
    </Alert>

    <Card v-if="generatedFiles.length > 0">
      <CardHeader>
        <CardTitle>生成结果</CardTitle>
        <CardDescription>共生成 {{ generatedFiles.length }} 个文件</CardDescription>
      </CardHeader>
      <CardContent>
        <ul class="space-y-2 rounded-lg border bg-muted/30 p-4 font-mono text-sm">
          <li v-for="file of generatedFiles" :key="file" class="break-all">
            {{ file }}
          </li>
        </ul>
      </CardContent>
    </Card>
  </div>
</template>

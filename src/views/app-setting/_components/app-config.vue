<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Save, RotateCcw, FolderOpen } from 'lucide-vue-next';
import { open } from '@tauri-apps/plugin-dialog';
import ThemeToggle from '@/components/theme/theme-toggle.vue';
import { storeToRefs } from 'pinia';
import { useAppStore } from '@/stores/app';
import { toast } from 'vue-sonner';

const appStore = useAppStore();
const { settings } = storeToRefs(appStore);
const { updateSettings, resetSettings, saveToRust } = appStore;
const projectFilesMaxFileContentLengthModel = computed({
  get: () => settings.value.projectFilesMaxFileContentLength,
  set: (value: string | number) => {
    updateSettings({
      projectFilesMaxFileContentLength: Math.max(1, Number(value) || 15000),
    });
  },
});

async function save() {
  await saveToRust();
  toast.success('应用设置已保存');
}

function reset() {
  resetSettings();
}

// 选择项目文件夹
async function selectProjectFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择项目文件夹',
  });

  if (selected && typeof selected === 'string') {
    updateSettings({ projectPath: selected });
    toast.success('项目文件夹已设置');
  }
}

// 清除项目文件夹
function clearProjectFolder() {
  updateSettings({ projectPath: null });
  toast.success('项目文件夹已清除');
}

async function selectStorageFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择应用存储文件夹',
  });

  if (selected && typeof selected === 'string') {
    updateSettings({ storageDir: selected });
    toast.success('存储文件夹已设置');
  }
}

function clearStorageFolder() {
  updateSettings({ storageDir: null });
  toast.success('已恢复默认存储目录');
}
</script>

<template>
  <Card class="h-full">
    <CardHeader>
      <CardTitle class="text-lg">
        应用设置
      </CardTitle>
      <CardDescription>
        个性化应用体验
      </CardDescription>
    </CardHeader>

    <ScrollArea class="flex-1 min-h-0">
      <CardContent class="space-y-4 ">
        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>自动保存对话</Label>
            <p class="text-sm text-muted-foreground">
              自动保存对话历史到本地存储
            </p>
          </div>
          <Switch
            :model-value="settings.autoSave"
            @update:model-value="updateSettings({ autoSave: Boolean($event) })"
          />
        </div>

        <Separator />

        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>主题设置</Label>
            <p class="text-sm text-muted-foreground">
              选择应用主题外观
            </p>
          </div>
          <ThemeToggle />
        </div>

        <Separator />

        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>项目文件夹</Label>
            <p class="text-sm text-muted-foreground">
              选择本地项目文件夹路径
            </p>
          </div>
          <div class="flex items-center gap-2">
            <Input
              :model-value="settings.projectPath || ''"
              placeholder="未选择文件夹"
              class="w-64"
              readonly
            />
            <Button
              v-if="settings.projectPath"
              variant="outline"
              size="sm"
              @click="clearProjectFolder"
            >
              清除
            </Button>
            <Button
              size="sm"
              @click="selectProjectFolder"
            >
              <FolderOpen class="h-4 w-4 mr-2" />
              选择
            </Button>
          </div>
        </div>

        <Separator />

        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>存储文件夹</Label>
            <p class="text-sm text-muted-foreground">
              统一存放内置数据库和 Agent 记忆数据
            </p>
          </div>
          <div class="flex items-center gap-2">
            <Input
              :model-value="settings.storageDir || ''"
              placeholder="默认使用应用系统目录"
              class="w-64"
              readonly
            />
            <Button
              v-if="settings.storageDir"
              variant="outline"
              size="sm"
              @click="clearStorageFolder"
            >
              清除
            </Button>
            <Button
              size="sm"
              @click="selectStorageFolder"
            >
              <FolderOpen class="h-4 w-4 mr-2" />
              选择
            </Button>
          </div>
        </div>

        <Separator />

        <div class="space-y-4">
          <div class="space-y-0.5">
            <Label>项目文件设置</Label>
            <p class="text-sm text-muted-foreground">
              配置代码分析页扫描文件时使用的扩展名、忽略目录和单文件读取上限
            </p>
          </div>

          <div class="grid gap-4 lg:grid-cols-2">
            <div class="space-y-2">
              <Label>扫描扩展名</Label>
              <Input
                :model-value="settings.projectFilesExtensions"
                placeholder=".ts,.tsx,.js,.jsx,.vue"
                @update:model-value="updateSettings({ projectFilesExtensions: String($event ?? '') })"
              />
              <p class="text-xs text-muted-foreground">
                使用逗号分隔，例如 `.ts,.tsx,.js`
              </p>
            </div>

            <div class="space-y-2">
              <Label>忽略目录</Label>
              <Input
                :model-value="settings.projectFilesIgnoreDirs"
                placeholder="node_modules,.git,dist"
                @update:model-value="updateSettings({ projectFilesIgnoreDirs: String($event ?? '') })"
              />
              <p class="text-xs text-muted-foreground">
                使用逗号分隔，例如 `node_modules,.git,dist`
              </p>
            </div>
          </div>

          <div class="space-y-2">
            <Label>单文件最大读取字符数</Label>
            <Input
              v-model="projectFilesMaxFileContentLengthModel"
              type="number"
              min="1"
              step="1"
            />
            <p class="text-xs text-muted-foreground">
              控制代码分析页读取单个文件内容时的最大字符数
            </p>
          </div>
        </div>

        <Separator />
      </CardContent>
    </ScrollArea>

    <CardFooter class="flex justify-between">
      <Button variant="outline" @click="reset">
        <RotateCcw class="h-4 w-4 mr-2" />
        重置默认
      </Button>
      <Button @click="save">
        <Save class="h-4 w-4 mr-2" />
        保存设置
      </Button>
    </CardFooter>
  </Card>
</template>

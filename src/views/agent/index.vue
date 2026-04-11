<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import AgentLayout from './components/agent-layout.vue';
import ChatPanel from './components/ChatPanel.vue';
import ProjectFileTree from './components/ProjectFileTree.vue';
import type { AgentMessage } from '@/types';
import type { ProjectFileEntry, ProjectFilesSettings } from '@/types/project-files';
import { readProjectFile, scanProjectFiles } from '@/services/project-files';
import { getErrorMessage } from '@/utils/helpers';
import { CircleAlert } from 'lucide-vue-next';

const files = ref<ProjectFileEntry[]>([]);
const selectedFilePath = ref<string | null>(null);
const selectedFileContent = ref('');
const messages = ref<AgentMessage[]>([
  createAssistantMessage([
    '## Mock 代码分析对话',
    '',
    '这里当前保留的是 mock 对话，但左侧文件树和当前文件内容已经接回真实项目文件能力。',
    '',
    '你可以：',
    '- 在左侧查看并点选真实项目文件',
    '- 在右侧发送问题',
    '- 查看基于当前真实文件内容生成的示例回答',
  ].join('\n')),
]);
const isAnalyzing = ref(false);
const isLoadingFiles = ref(false);
let responseTimer: number | null = null;

const projectPath = ref('');
const projectFilesSettings = computed<ProjectFilesSettings>(() => ({
  fileExtensions: '.ts,.tsx,.js,.jsx,.vue',
  ignoreDirs: 'node_modules,.git,dist',
  maxFileContentLength: 15000,
}));

function createUserMessage(content: string): AgentMessage {
  return {
    id: `user-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role: 'user',
    content,
    createdAt: Date.now(),
    status: 'done',
  };
}

function createAssistantMessage(content: string): AgentMessage {
  return {
    id: `assistant-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role: 'assistant',
    content,
    createdAt: Date.now(),
    status: 'done',
  };
}

async function loadFiles(options?: { preserveSelection?: boolean }) {
  if (!projectPath.value) {
    files.value = [];
    selectedFilePath.value = null;
    selectedFileContent.value = '';
    return;
  }

  isLoadingFiles.value = true;

  try {
    const nextFiles = await scanProjectFiles(projectPath.value, projectFilesSettings.value);
    files.value = nextFiles;

    const preserveSelection = options?.preserveSelection ?? true;
    const nextSelectedPath = preserveSelection
      ? nextFiles.find(file => file.path === selectedFilePath.value)?.path ?? nextFiles[0]?.path ?? null
      : nextFiles[0]?.path ?? null;

    selectedFilePath.value = nextSelectedPath;
    if (nextSelectedPath) {
      await openFile(nextSelectedPath);
    } else {
      selectedFileContent.value = '';
    }
  } finally {
    isLoadingFiles.value = false;
  }
}

async function openFile(path: string) {
  if (!projectPath.value) {
    return;
  }

  selectedFilePath.value = path;
  selectedFileContent.value = await readProjectFile({
    projectPath: projectPath.value,
    filePath: path,
    maxChars: projectFilesSettings.value.maxFileContentLength,
  });
}

async function handleOpenFile(path: string) {
  try {
    await openFile(path);
  } catch (error) {
    toast.error(getErrorMessage(error, '读取项目文件失败'));
  }
}

async function handleRefresh() {
  try {
    await loadFiles();
    toast.success('项目文件已刷新');
  } catch (error) {
    toast.error(getErrorMessage(error, '刷新项目文件失败'));
  }
}

function handleSend(content: string) {
  const trimmed = content.trim();
  if (!trimmed || isAnalyzing.value) {
    return;
  }

  messages.value = [
    ...messages.value,
    createUserMessage(trimmed),
  ];
  isAnalyzing.value = true;

  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }

  responseTimer = window.setTimeout(() => {
    messages.value = [
      ...messages.value,
      createAssistantMessage(buildMockReply(trimmed)),
    ];
    isAnalyzing.value = false;
    responseTimer = null;
  }, 450);
}

function buildMockReply(question: string) {
  const currentFile = selectedFilePath.value ?? '未选中文件';
  const fileContent = selectedFileContent.value || '当前没有可用文件内容';
  const filePreview = fileContent.split('\n').slice(0, 6).join('\n');

  return [
    `## Mock 分析结果`,
    '',
    `- 当前问题：${question}`,
    `- 当前上下文文件：\`${currentFile}\``,
    `- 数据来源：当前项目文件`,
    '',
    `### 文件内容预览`,
    '```ts',
    filePreview,
    '```',
    '',
    `### 模拟结论`,
    `我现在不会调用真实聊天 Agent，只会基于当前选中的真实文件内容给出示例回答。根据 \`${currentFile}\` 的内容来看，这个问题更可能和当前文件实现有关，后面你接手时可以把这里替换成真实分析链路。`,
  ].join('\n');
}

async function initializeProjectFiles() {
  if (!projectPath.value) {
    files.value = [];
    selectedFilePath.value = null;
    selectedFileContent.value = '';
    return;
  }

  try {
    await loadFiles({ preserveSelection: false });
  } catch (error) {
    toast.error(getErrorMessage(error, '加载项目文件失败'));
  }
}

onMounted(() => {
  void initializeProjectFiles();
});

watch(projectPath, () => {
  void initializeProjectFiles();
});

onBeforeUnmount(() => {
  if (responseTimer) {
    window.clearTimeout(responseTimer);
  }
});
</script>

<template>
  <div class="h-screen bg-background relative">
    <div v-if="!projectPath" class="warning-alert">
      <Alert>
        <CircleAlert class="h-4 w-4" />
        <AlertTitle>项目目录设置正在重构</AlertTitle>
        <AlertDescription>
          代码分析页依赖的项目目录配置已暂时下线，等新的通用设置方案确定后再接回。
        </AlertDescription>
      </Alert>
    </div>

    <AgentLayout v-else>
      <template #files>
        <ProjectFileTree
          :files="files"
          :selected-path="selectedFilePath"
          @select="handleOpenFile"
          @refresh="handleRefresh"
        />
      </template>

      <template #chat>
        <ChatPanel
          :messages="messages"
          :selected-file-path="selectedFilePath"
          :selected-file-content="selectedFileContent"
          :busy="isAnalyzing || isLoadingFiles"
          @submit="handleSend"
        />
      </template>
    </AgentLayout>
  </div>
</template>

<style scoped>
.warning-alert {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}
</style>

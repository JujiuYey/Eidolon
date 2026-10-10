<script setup lang="ts">
import type { PmSkillSummary } from '@/types/pm';
import MarkdownRender from 'markstream-vue';
import { open } from '@tauri-apps/plugin-dialog';
import { Brain, FolderOpen, Search, Trash2 } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import {
  getPmSettings,
  getPmSkillContent,
  listPmSkills,
  revealDirectory,
  upsertPmSettings,
} from '@/services/pm';
import { getErrorMessage } from '@/utils/helpers';

const skills = ref<PmSkillSummary[]>([]);
const isLoading = ref(false);
const isPicking = ref(false);

const overrideDir = ref('');
const searchKeyword = ref('');

const previewOpen = ref(false);
const previewSkill = ref<PmSkillSummary | null>(null);
const previewContent = ref('');
const previewLoading = ref(false);

const overrideCount = computed(() => skills.value.filter(skill => skill.source === 'override').length);

const filteredSkills = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return skills.value;
  }
  return skills.value.filter(skill =>
    `${skill.slug} ${skill.name} ${skill.description}`.toLowerCase().includes(keyword),
  );
});

function formatSize(size: number): string {
  return size >= 1024 ? `${(size / 1024).toFixed(1)} KB` : `${size} B`;
}

async function loadSkills() {
  isLoading.value = true;
  try {
    const [list, settings] = await Promise.all([listPmSkills(), getPmSettings()]);
    skills.value = list;
    overrideDir.value = settings.skills_override_dir ?? '';
  } catch (error) {
    toast.error(getErrorMessage(error, '技能库加载失败'));
  } finally {
    isLoading.value = false;
  }
}

async function pickOverrideDir() {
  isPicking.value = true;
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择 Skills 覆盖目录（如 awesome-ux-skills 的本地 clone）',
    });
    if (typeof selected === 'string' && selected) {
      await saveOverrideDir(selected);
    }
  } finally {
    isPicking.value = false;
  }
}

async function saveOverrideDir(dir: string | null) {
  try {
    const saved = await upsertPmSettings({ skills_override_dir: dir });
    overrideDir.value = saved.skills_override_dir ?? '';
    await loadSkills();
    toast.success(dir ? '覆盖目录已保存，技能库已重新加载' : '已恢复使用内置技能库');
  } catch (error) {
    toast.error(getErrorMessage(error, '覆盖目录保存失败'));
  }
}

async function openPreview(skill: PmSkillSummary) {
  previewSkill.value = skill;
  previewContent.value = '';
  previewOpen.value = true;
  previewLoading.value = true;
  try {
    previewContent.value = await getPmSkillContent(skill.slug);
  } catch (error) {
    toast.error(getErrorMessage(error, '框架内容加载失败'));
    previewOpen.value = false;
  } finally {
    previewLoading.value = false;
  }
}

onMounted(() => {
  void loadSkills();
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card">
    <div class="flex items-center gap-2 px-4 py-3">
      <Brain class="h-4 w-4 text-primary" />
      <h2 class="text-sm font-semibold">
        Skills
      </h2>
    </div>
    <Separator />

    <ScrollArea class="min-h-0 flex-1">
      <div class="px-5 py-1">
        <!-- 覆盖目录 -->
        <section class="py-3">
          <h3 class="text-sm font-semibold">
            覆盖目录
          </h3>
          <p class="mt-1 text-sm text-muted-foreground">
            产品经理分身的方法论框架库随应用内置；如需自行更新（例如 awesome-ux-skills 有新版），把仓库
            clone 到本地后在这里指向它。目录下每个子文件夹里的 SKILL.md 会按文件夹名覆盖内置版本，
            也能追加全新框架。
          </p>

          <div class="mt-4 flex max-w-[520px] items-center gap-2">
            <Input
              :model-value="overrideDir"
              aria-label="覆盖目录路径"
              class="font-mono text-xs"
              readonly
            />
            <Button
              :disabled="isPicking"
              size="sm"
              variant="outline"
              @click="pickOverrideDir"
            >
              <FolderOpen class="h-4 w-4" />
              浏览
            </Button>
            <Button
              v-if="overrideDir"
              aria-label="在文件管理器中打开"
              size="sm"
              variant="outline"
              @click="revealDirectory(overrideDir)"
            >
              <FolderOpen class="h-4 w-4" />
            </Button>
            <Button
              v-if="overrideDir"
              aria-label="清除覆盖目录"
              size="sm"
              variant="outline"
              @click="saveOverrideDir(null)"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </section>

        <Separator />

        <!-- 框架清单 -->
        <section class="py-3">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 class="text-sm font-semibold">
                方法论框架库
              </h3>
              <p class="mt-1 text-xs text-muted-foreground">
                共 {{ skills.length }} 个框架<span v-if="overrideCount > 0">，其中 {{ overrideCount }} 个来自覆盖目录</span>。点击条目查看完整方法文本。
              </p>
            </div>
            <div class="relative">
              <Search class="pointer-events-none absolute top-1/2 left-2.5 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                v-model="searchKeyword"
                aria-label="搜索框架"
                class="h-9 w-56 pl-8"
                placeholder="搜索框架名称或描述"
              />
            </div>
          </div>

          <div class="mt-4 space-y-2">
            <template v-if="isLoading">
              <Skeleton
                v-for="index of 5"
                :key="index"
                class="h-14"
              />
            </template>

            <p
              v-else-if="filteredSkills.length === 0"
              class="py-6 text-center text-sm text-muted-foreground"
            >
              {{ skills.length === 0 ? '技能库为空，请检查应用资源是否完整。' : '没有匹配的框架。' }}
            </p>

            <template v-else>
              <button
                v-for="skill of filteredSkills"
                :key="skill.slug"
                type="button"
                class="block w-full rounded-xl border bg-background/40 px-4 py-3 text-left transition-colors hover:bg-muted/60"
                @click="openPreview(skill)"
              >
                <div class="flex items-center gap-2">
                  <span class="truncate font-mono text-sm font-medium">
                    {{ skill.slug }}
                  </span>
                  <Badge
                    :variant="skill.source === 'override' ? 'default' : 'secondary'"
                    class="text-[11px]"
                  >
                    {{ skill.source === 'override' ? '覆盖' : '内置' }}
                  </Badge>
                  <span class="ml-auto shrink-0 text-xs text-muted-foreground tabular-nums">
                    {{ formatSize(skill.size) }}
                  </span>
                </div>
                <p class="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
                  {{ skill.description || '（无描述）' }}
                </p>
              </button>
            </template>
          </div>
        </section>
      </div>
    </ScrollArea>
  </div>

  <!-- 框架全文预览 -->
  <Dialog
    :open="previewOpen"
    @update:open="value => previewOpen = value"
  >
    <DialogContent class="flex max-h-[80vh] flex-col sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle class="font-mono">
          {{ previewSkill?.slug }}
        </DialogTitle>
        <DialogDescription>
          {{ previewSkill?.description || '（无描述）' }}
        </DialogDescription>
      </DialogHeader>

      <ScrollArea class="min-h-0 flex-1">
        <div class="pr-3 text-sm">
          <div v-if="previewLoading">
            <Skeleton class="h-64 w-full" />
          </div>
          <div
            v-else
            class="pm-markdown"
          >
            <MarkdownRender
              :content="previewContent"
              custom-id="pm-skill-preview"
            />
          </div>
        </div>
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>

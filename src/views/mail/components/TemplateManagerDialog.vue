<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Plus, Trash2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { useMailStore } from '@/stores/mail';
import { extractVariables } from '@/views/mail/utils/template-helpers';
import { getErrorMessage } from '@/utils/helpers';

const open = defineModel<boolean>('open', { required: true });

const store = useMailStore();

const editingId = ref('');
/** 空字符串表示正在新建 */
const form = ref({ id: '', name: '', subject: '', body: '' });
const deleteConfirmOpen = ref(false);
const saving = ref(false);

const isCreating = computed(() => form.value.id === '');
const formVariables = computed(() =>
  extractVariables(form.value.subject, form.value.body),
);
const canSave = computed(() => Boolean(form.value.name.trim()));

watch(open, isOpen => {
  if (isOpen) {
    const first = store.templates[0];
    if (first) {
      startEdit(first.id);
    } else {
      startCreate();
    }
  }
});

function startCreate() {
  editingId.value = '';
  form.value = { id: '', name: '', subject: '', body: '' };
}

function startEdit(templateId: string) {
  const template = store.templates.find(item => item.id === templateId);
  if (!template) {
    startCreate();
    return;
  }
  editingId.value = template.id;
  form.value = {
    id: template.id,
    name: template.name,
    subject: template.subject,
    body: template.body,
  };
}

async function handleSave() {
  if (!canSave.value) {
    return;
  }
  saving.value = true;
  try {
    const saved = await store.saveTemplate({
      id: form.value.id,
      name: form.value.name.trim(),
      subject: form.value.subject,
      body: form.value.body,
      sort: 0,
      created_at: 0,
      updated_at: 0,
    });
    toast.success('模板已保存');
    startEdit(saved.id);
  } catch (error) {
    toast.error(getErrorMessage(error, '保存模板失败'));
  } finally {
    saving.value = false;
  }
}

async function handleDelete() {
  if (!form.value.id) {
    return;
  }
  try {
    await store.removeTemplate(form.value.id);
    deleteConfirmOpen.value = false;
    toast.success('模板已删除');
    if (store.templates[0]) {
      startEdit(store.templates[0].id);
    } else {
      startCreate();
    }
  } catch (error) {
    toast.error(getErrorMessage(error, '删除模板失败'));
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-4xl">
      <DialogHeader>
        <DialogTitle>管理邮件模板</DialogTitle>
        <DialogDescription>
          模板中用 <code
            v-pre
            class="rounded bg-muted px-1"
          >{{变量名}}</code> 声明占位符（支持中文），写信时会生成对应表单。
        </DialogDescription>
      </DialogHeader>

      <div class="flex min-h-0 gap-4">
        <div class="flex w-48 shrink-0 flex-col gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            class="justify-start"
            :class="isCreating && 'border-primary bg-primary/20 shadow-xs'"
            @click="startCreate"
          >
            <Plus class="mr-1 h-4 w-4" />
            新建模板
          </Button>
          <ScrollArea class="max-h-80">
            <div class="flex flex-col gap-1 pr-2">
              <button
                v-for="template of store.templates"
                :key="template.id"
                type="button"
                class="truncate rounded-md px-2 py-1.5 text-left text-sm transition-colors"
                :class="editingId === template.id
                  ? 'bg-primary/15 font-medium text-foreground'
                  : 'text-muted-foreground hover:bg-muted'"
                @click="startEdit(template.id)"
              >
                {{ template.name }}
              </button>
            </div>
          </ScrollArea>
        </div>

        <Separator
          orientation="vertical"
          class="hidden h-auto md:block"
        />

        <div class="min-w-0 flex-1 space-y-4">
          <div class="space-y-2">
            <Label class="text-sm font-medium">模板名称</Label>
            <Input
              v-model="form.name"
              placeholder="例如: 请假申请"
              class="h-10"
            />
          </div>

          <div class="space-y-2">
            <Label class="text-sm font-medium">主题</Label>
            <Input
              v-model="form.subject"
              placeholder="例如: {{姓名}}的{{请假类型}}申请"
              class="h-10"
            />
          </div>

          <div class="space-y-2">
            <Label class="text-sm font-medium">正文</Label>
            <Textarea
              v-model="form.body"
              placeholder="邮件正文，可使用 {{变量}}"
              class="min-h-56 font-mono text-[13px] leading-6"
            />
          </div>

          <p
            v-if="formVariables.length > 0"
            class="text-xs text-muted-foreground"
          >
            已识别变量：{{ formVariables.join('、') }}
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-3">
        <Button
          v-if="!isCreating"
          type="button"
          variant="ghost"
          class="text-destructive hover:text-destructive"
          @click="deleteConfirmOpen = true"
        >
          <Trash2 class="mr-2 h-4 w-4" />
          删除模板
        </Button>
        <Button
          type="button"
          variant="outline"
          @click="open = false"
        >
          关闭
        </Button>
        <Button
          type="button"
          :disabled="!canSave || saving"
          @click="handleSave"
        >
          {{ saving ? '保存中…' : '保存模板' }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除该模板吗？"
    description="删除后发送历史仍会保留，但无法再使用该模板写信。"
    type="destructive"
    @confirm="handleDelete"
  />
</template>

<script setup lang="ts">
import type { PmConversation } from '@/types/pm';
import { MoreHorizontal, MessagesSquare, Pencil, Plus, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { formatRelativeTime } from '../utils/format';

interface Props {
  conversations: PmConversation[];
  activeId: string;
  isLoading: boolean;
}

interface Emits {
  (e: 'select', conversationId: string): void;
  (e: 'create'): void;
  (e: 'rename', conversationId: string, title: string): void;
  (e: 'remove', conversationId: string): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();

const removeOpen = ref(false);
const pendingConversation = ref<PmConversation | null>(null);

const renameOpen = ref(false);
const renameDraft = ref('');
const renameTarget = ref<PmConversation | null>(null);

function askRemove(conversation: PmConversation) {
  pendingConversation.value = conversation;
  removeOpen.value = true;
}

function confirmRemove() {
  if (pendingConversation.value) {
    emit('remove', pendingConversation.value.id);
  }
  removeOpen.value = false;
  pendingConversation.value = null;
}

function openRename(conversation: PmConversation) {
  renameTarget.value = conversation;
  renameDraft.value = conversation.title === '新对话' ? '' : conversation.title;
  renameOpen.value = true;
}

function confirmRename() {
  const target = renameTarget.value;
  const title = renameDraft.value.trim();
  if (target && title) {
    emit('rename', target.id, title);
  }
  renameOpen.value = false;
  renameTarget.value = null;
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col rounded-xl border bg-card">
    <div class="flex items-center justify-between gap-2 px-4 py-3">
      <h2 class="flex items-center gap-2 text-sm font-semibold">
        <MessagesSquare class="h-4 w-4 text-primary" />
        会话
      </h2>
      <Button
        size="sm"
        variant="outline"
        @click="emit('create')"
      >
        <Plus class="h-4 w-4" />
        新对话
      </Button>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-1 px-2 pb-2">
        <template v-if="isLoading">
          <Skeleton
            v-for="index of 4"
            :key="index"
            class="mx-2 h-12"
          />
        </template>

        <p
          v-else-if="conversations.length === 0"
          class="px-2 py-6 text-center text-xs text-muted-foreground"
        >
          还没有会话，点击「新对话」开始第一次讨论。
        </p>

        <template v-else>
          <div
            v-for="conversation of conversations"
            :key="conversation.id"
            class="group flex items-center gap-1 rounded-lg px-2 py-1.5 transition-colors hover:bg-muted/60"
            :class="conversation.id === activeId ? 'bg-muted' : ''"
          >
            <button
              type="button"
              class="min-w-0 flex-1 text-left"
              @click="emit('select', conversation.id)"
            >
              <p class="truncate text-sm font-medium">
                {{ conversation.title }}
              </p>
              <p class="mt-0.5 text-xs text-muted-foreground">
                {{ formatRelativeTime(conversation.updated_at) }}
              </p>
            </button>

            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button
                  aria-label="会话操作"
                  class="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                  size="icon"
                  variant="ghost"
                >
                  <MoreHorizontal class="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem @click="openRename(conversation)">
                  <Pencil class="mr-2 h-4 w-4" />
                  重命名
                </DropdownMenuItem>
                <DropdownMenuItem
                  class="text-destructive focus:text-destructive"
                  @click="askRemove(conversation)"
                >
                  <Trash2 class="mr-2 h-4 w-4" />
                  删除
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </template>
      </div>
    </ScrollArea>

    <SagConfirm
      v-model:open="removeOpen"
      :description="pendingConversation
        ? `将删除会话「${pendingConversation.title}」及其全部消息，删除后不可恢复。`
        : ''"
      title="删除会话"
      type="destructive"
      @confirm="confirmRemove"
    />

    <Dialog
      :open="renameOpen"
      @update:open="value => renameOpen = value"
    >
      <DialogContent class="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>重命名会话</DialogTitle>
          <DialogDescription>给会话起一个之后能认出来的名字。</DialogDescription>
        </DialogHeader>
        <Input
          v-model="renameDraft"
          aria-label="会话标题"
          maxlength="50"
          @keydown.enter="confirmRename"
        />
        <DialogFooter>
          <Button
            variant="outline"
            @click="renameOpen = false"
          >
            取消
          </Button>
          <Button
            :disabled="!renameDraft.trim()"
            @click="confirmRename"
          >
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>

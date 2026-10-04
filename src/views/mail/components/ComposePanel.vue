<script setup lang="ts">
import { computed } from 'vue';
import { toast } from 'vue-sonner';
import { SendHorizonal } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useMailStore } from '@/stores/mail';

const store = useMailStore();

const sending = computed(() => store.sendStatus === 'sending');

const missingVariables = computed(() =>
  store.composeVariables.filter(
    name => (store.variableValues[name] ?? '').trim() === '',
  ),
);

async function handleSend() {
  const outcome = await store.send();

  if (outcome.historyError) {
    toast.warning(outcome.historyError);
  }

  if (outcome.ok) {
    toast.success('邮件已发送');
  } else if (outcome.error) {
    toast.error(outcome.error);
  }
}
</script>

<template>
  <TooltipProvider :delay-duration="0">
    <div class="flex h-full flex-col">
      <ScrollArea class="min-h-0 flex-1">
        <div class="space-y-5 p-4">
          <section class="space-y-2">
            <Label class="text-sm font-semibold">收件人</Label>
            <Input
              :model-value="store.composeTo"
              placeholder="多个收件人用逗号分隔"
              @update:model-value="store.composeTo = String($event)"
            />
          </section>

          <section class="space-y-2">
            <Label class="text-sm font-semibold">抄送</Label>
            <Input
              :model-value="store.composeCc"
              placeholder="可选，多个抄送用逗号分隔"
              @update:model-value="store.composeCc = String($event)"
            />
          </section>

          <section
            v-if="store.composeVariables.length > 0"
            class="space-y-2"
          >
            <div class="flex items-center justify-between">
              <Label class="text-sm font-semibold">模板变量</Label>
              <span
                v-if="missingVariables.length > 0"
                class="text-xs text-amber-600 dark:text-amber-400"
              >
                待填写：{{ missingVariables.join('、') }}
              </span>
            </div>
            <div class="grid gap-3 rounded-xl border bg-muted/20 p-4 md:grid-cols-2">
              <div
                v-for="variable of store.composeVariables"
                :key="variable"
                class="space-y-1.5"
              >
                <Label
                  :for="`variable-${variable}`"
                  class="text-xs text-muted-foreground"
                >
                  {{ variable }}
                </Label>
                <Input
                  :id="`variable-${variable}`"
                  v-model="store.variableValues[variable]"
                  :placeholder="`请输入${variable}`"
                  class="h-9"
                />
              </div>
            </div>
          </section>

          <section class="space-y-2">
            <Label class="text-sm font-semibold">主题</Label>
            <Input
              :model-value="store.subject"
              placeholder="邮件主题"
              @update:model-value="store.setSubject(String($event))"
            />
          </section>

          <section class="space-y-2">
            <Label class="text-sm font-semibold">正文</Label>
            <Textarea
              :model-value="store.body"
              placeholder="邮件正文"
              class="min-h-[280px] font-mono text-[13px] leading-6"
              @update:model-value="store.setBody(String($event))"
            />
          </section>
        </div>
      </ScrollArea>

      <Separator />
      <div class="flex items-center justify-between gap-3 p-4">
        <p class="truncate text-xs text-muted-foreground">
          <template v-if="!store.selectedAccountId">
            尚未选择发件账户，可在左侧切换或前往 设置 → 邮件账户 配置。
          </template>
          <template v-else>
            使用 <span class="text-foreground">{{ store.selectedAccount?.email }}</span> 发送
          </template>
        </p>
        <Button
          type="button"
          class="shrink-0"
          :disabled="sending"
          @click="handleSend"
        >
          <SendHorizonal class="mr-2 h-4 w-4" />
          {{ sending ? '发送中…' : '发送' }}
        </Button>
      </div>
    </div>
  </TooltipProvider>
</template>

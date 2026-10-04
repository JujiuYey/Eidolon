<script lang="ts" setup>
import { refDebounced } from '@vueuse/core';
import { Search } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { cn } from '@/lib/utils';
import { Input } from '@/components/ui/input';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsContent } from '@/components/ui/tabs';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useMailStore } from '@/stores/mail';
import type { SentEmail } from '@/types/mail';
import AccountSwitcher from './AccountSwitcher.vue';
import ComposePanel from './ComposePanel.vue';
import Nav from './Nav.vue';
import type { LinkProp } from './Nav.vue';
import SentDisplay from './SentDisplay.vue';
import SentList from './SentList.vue';
import TemplateManagerDialog from './TemplateManagerDialog.vue';
import TemplatePicker from './TemplatePicker.vue';

const store = useMailStore();
const router = useRouter();

const isCollapsed = ref(false);
const templateManagerOpen = ref(false);
const searchValue = ref('');
const debouncedSearch = refDebounced(searchValue, 250);

const filteredSentList = computed(() => {
  const keyword = debouncedSearch.value?.trim().toLowerCase();
  if (!keyword) {
    return store.sentEmails;
  }

  return store.sentEmails.filter((email: SentEmail) =>
    email.subject.toLowerCase().includes(keyword)
    || email.to_addresses.toLowerCase().includes(keyword)
    || email.account_email.toLowerCase().includes(keyword)
    || email.body.toLowerCase().includes(keyword),
  );
});

const navLinks = computed<LinkProp[]>(() => [
  {
    title: '写信',
    icon: 'lucide:pen-line',
    variant: store.viewMode === 'compose' ? 'default' : 'ghost',
    onSelect: () => {
      store.viewMode = 'compose';
    },
  },
  {
    title: '已发送',
    icon: 'lucide:send',
    label: store.sentEmails.length > 0 ? String(store.sentEmails.length) : '',
    variant: store.viewMode === 'sent' ? 'default' : 'ghost',
    onSelect: () => {
      store.viewMode = 'sent';
    },
  },
  {
    title: '模板管理',
    icon: 'lucide:layout-template',
    variant: 'ghost',
    onSelect: () => {
      templateManagerOpen.value = true;
    },
  },
  {
    title: '管理账户',
    icon: 'lucide:settings',
    variant: 'ghost',
    onSelect: () => {
      router.push('/app-setting');
    },
  },
]);

function handleResend() {
  const mail = store.selectedSentEmail;
  if (mail) {
    store.resend(mail);
  }
}
</script>

<template>
  <TooltipProvider :delay-duration="0">
    <ResizablePanelGroup
      id="mail-panel-group"
      direction="horizontal"
      class="h-full items-stretch"
    >
      <ResizablePanel
        id="mail-nav-panel"
        :default-size="20"
        :collapsed-size="6"
        collapsible
        :min-size="12"
        :max-size="20"
        :class="cn(isCollapsed && 'min-w-[50px] transition-all duration-300 ease-in-out')"
        @collapse="isCollapsed = true"
        @expand="isCollapsed = false"
      >
        <div class="flex flex-col gap-2 p-2">
          <AccountSwitcher
            v-model:selected-account-id="store.selectedAccountId"
            :is-collapsed="isCollapsed"
            :accounts="store.accounts"
          />
        </div>
        <Separator />
        <Nav
          :is-collapsed="isCollapsed"
          :links="navLinks"
        />
      </ResizablePanel>
      <ResizableHandle id="mail-nav-handle" with-handle />

      <!-- 中栏：写信时选模板，已发送时列历史 -->
      <ResizablePanel id="mail-list-panel" :default-size="30" :min-size="25">
        <template v-if="store.viewMode === 'compose'">
          <div class="flex items-center px-4 py-3.5">
            <h1 class="text-xl font-bold">
              写信
            </h1>
          </div>
          <Separator />
          <TemplatePicker
            v-model:selected-template-id="store.selectedTemplateId"
            :templates="store.templates"
            @manage="templateManagerOpen = true"
          />
        </template>

        <Tabs
          v-else
          default-value="all"
        >
          <div class="flex items-center px-4 py-2">
            <h1 class="text-xl font-bold">
              已发送
            </h1>
          </div>
          <Separator />
          <div class="bg-background/95 p-4 backdrop-blur supports-[backdrop-filter]:bg-background/60">
            <form>
              <div class="relative">
                <Search class="absolute left-2 top-2.5 size-4 text-muted-foreground" />
                <Input v-model="searchValue" placeholder="搜索主题、收件人或正文" class="pl-8" />
              </div>
            </form>
          </div>
          <TabsContent value="all" class="m-0">
            <SentList v-model:selected-id="store.selectedSentEmailId" :items="filteredSentList" />
          </TabsContent>
        </Tabs>
      </ResizablePanel>
      <ResizableHandle id="mail-display-handle" with-handle />

      <!-- 右栏：写信表单或发送详情 -->
      <ResizablePanel id="mail-display-panel" :default-size="50">
        <ComposePanel v-if="store.viewMode === 'compose'" />
        <SentDisplay
          v-else
          :mail="store.selectedSentEmail"
          @resend="handleResend"
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  </TooltipProvider>

  <TemplateManagerDialog v-model:open="templateManagerOpen" />
</template>

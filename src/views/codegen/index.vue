<script setup lang="ts">
import { ref } from 'vue';
import { Sparkles } from 'lucide-vue-next';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import BackendGoCrudGenerator from './components/backend-go-crud-generator.vue';
import FrontendCrudGenerator from './components/frontend-crud-generator.vue';

const activeTab = ref<'backend' | 'frontend'>('backend');
</script>

<template>
  <div class="mx-auto flex h-screen max-w-6xl flex-col overflow-hidden p-6">
    <div class="mb-6 shrink-0 flex flex-col gap-3 border-b pb-4 lg:flex-row lg:items-end lg:justify-between">
      <div>
        <h1 class="mb-2 flex items-center gap-2 text-2xl font-bold">
          <Sparkles class="h-6 w-6 text-primary" />
          CRUD Code Generator
        </h1>
        <p class="text-muted-foreground">
          在一个入口里同时支持 Go 后端 CRUD 代码生成和前端 CRUD 页面生成。
        </p>
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1">
      <Tabs v-model="activeTab" class="space-y-6 pr-4">
        <TabsList class="grid w-full max-w-md grid-cols-2">
          <TabsTrigger value="backend">
            后端 Go CRUD
          </TabsTrigger>
          <TabsTrigger value="frontend">
            前端 CRUD
          </TabsTrigger>
        </TabsList>

        <TabsContent value="backend" class="mt-0">
          <BackendGoCrudGenerator />
        </TabsContent>

        <TabsContent value="frontend" class="mt-0">
          <FrontendCrudGenerator />
        </TabsContent>
      </Tabs>
    </ScrollArea>
  </div>
</template>

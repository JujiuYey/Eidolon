<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Save, RotateCcw } from 'lucide-vue-next';
import ThemeToggle from './theme-toggle.vue';
import { useAppStore } from '@/stores/app';
import { toast } from 'vue-sonner';

const appStore = useAppStore();
const { resetSettings, saveToRust } = appStore;

async function save() {
  await saveToRust();
  toast.success('应用设置已保存');
}

function reset() {
  resetSettings();
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
            <Label>主题设置</Label>
            <p class="text-sm text-muted-foreground">
              选择应用主题外观
            </p>
          </div>
          <ThemeToggle />
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

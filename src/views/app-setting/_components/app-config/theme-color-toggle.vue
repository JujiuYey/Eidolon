<script setup lang="ts">
import { cn } from '@/lib/utils';
import { useTheme } from '@/composables/use-theme';
import { THEME_COLOR_OPTIONS } from '@/utils/theme';

const { themeColor, setThemeColor } = useTheme();
</script>

<template>
  <TooltipProvider>
    <div class="flex flex-wrap items-center justify-end gap-2">
      <Tooltip v-for="option of THEME_COLOR_OPTIONS" :key="option.value">
        <TooltipTrigger as-child>
          <button
            type="button"
            :aria-label="`切换到${option.label}`"
            :aria-pressed="themeColor === option.value"
            :class="
              cn(
                'flex h-7 w-7 items-center justify-center rounded-full border border-background/80 shadow-sm transition-all outline-none hover:scale-105 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
                themeColor === option.value && 'scale-105 ring-2 ring-ring ring-offset-2 ring-offset-background',
              )
            "
            :style="{ backgroundColor: option.preview }"
            @click="setThemeColor(option.value)"
          >
            <span
              :class="cn('h-2.5 w-2.5 rounded-full transition-opacity', themeColor === option.value ? 'opacity-100' : 'opacity-0')"
              :style="{ backgroundColor: option.indicator }"
            />
            <span class="sr-only">{{ option.label }}</span>
          </button>
        </TooltipTrigger>

        <TooltipContent>
          {{ option.label }}
        </TooltipContent>
      </Tooltip>
    </div>
  </TooltipProvider>
</template>

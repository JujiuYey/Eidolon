<script lang="ts" setup>
import type { AcceptableValue } from 'reka-ui';
import { X } from 'lucide-vue-next';
import { isNil } from 'es-toolkit';

interface Option {
  label: string;
  value: AcceptableValue;
  icon?: string;
  disabled?: boolean;
}

interface Props {
  modelValue?: AcceptableValue;
  options?: Option[];
  placeholder?: string;
  clearable?: boolean;
  disabled?: boolean;
  size?: 'sm' | 'default';
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  options: () => [],
  placeholder: '请选择',
  clearable: true,
  disabled: false,
  size: 'default',
});

const emit = defineEmits<{
  (e: 'update:modelValue', value: AcceptableValue): void;
  (e: 'change', value: AcceptableValue): void;
}>();

// 计算属性用于双向绑定
const selectedValue = computed({
  get: () => props.modelValue,
  set: (value: AcceptableValue) => {
    emit('update:modelValue', value);
    emit('change', value);
  },
});

// 清除选择
function clearSelection() {
  selectedValue.value = null;
}

// 获取当前选中项
const selectedOption = computed(() => {
  return props.options.find(option => option.value === props.modelValue);
});
</script>

<template>
  <div class="relative flex items-center gap-2">
    <Select v-model="selectedValue" :disabled="disabled">
      <SelectTrigger :size="size" class="flex-1">
        <SelectValue :placeholder="placeholder">
          <span v-if="selectedOption" class="flex items-center gap-2">
            <span v-if="selectedOption.icon">{{ selectedOption.icon }}</span>
            {{ selectedOption.label }}
          </span>
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectItem
            v-for="option of options"
            :key="String(option.value)"
            :value="option.value"
            :disabled="option.disabled"
          >
            <span class="flex items-center gap-2">
              <span v-if="option.icon">{{ option.icon }}</span>
              {{ option.label }}
            </span>
          </SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>

    <!-- 清除按钮 -->
    <Button
      v-if="clearable && !isNil(modelValue)"
      variant="outline"
      size="icon"
      class="shrink-0"
      :disabled="disabled"
      @click="clearSelection"
    >
      <X class="h-4 w-4" />
    </Button>
  </div>
</template>

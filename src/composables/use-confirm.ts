import { reactive } from 'vue';

export interface ConfirmOptions {
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

interface PendingConfirm {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  destructive: boolean;
  resolve: ((value: boolean) => void) | null;
}

const state = reactive<PendingConfirm>({
  open: false,
  title: '',
  description: '',
  confirmLabel: '确认',
  cancelLabel: '取消',
  destructive: false,
  resolve: null,
});

export function useConfirm() {
  function ask(options: ConfirmOptions): Promise<boolean> {
    state.title = options.title;
    state.description = options.description;
    state.confirmLabel = options.confirmLabel ?? '确认';
    state.cancelLabel = options.cancelLabel ?? '取消';
    state.destructive = options.destructive ?? false;
    state.resolve = null;
    state.open = true;
    return new Promise<boolean>(resolve => {
      state.resolve = resolve;
    });
  }

  function onOpenChange(open: boolean): void {
    state.open = open;
    if (!open) {
      state.resolve?.(false);
      state.resolve = null;
    }
  }

  function onConfirm(): void {
    state.resolve?.(true);
    state.resolve = null;
    state.open = false;
  }

  function onCancel(): void {
    state.resolve?.(false);
    state.resolve = null;
    state.open = false;
  }

  return { state, ask, onOpenChange, onConfirm, onCancel };
}

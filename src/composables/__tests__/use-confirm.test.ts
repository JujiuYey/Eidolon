import { strict as assert } from 'node:assert';
import { describe, it } from 'vitest';
import { useConfirm } from '@/composables/use-confirm';

describe('useConfirm', () => {
  it('opens the dialog and resolves true on confirm', async () => {
    const c = useConfirm();
    const promise = c.ask({
      title: '删除项目',
      description: '确认要删除吗？',
      confirmLabel: '删除',
      destructive: true,
    });

    assert.equal(c.state.open, true);
    assert.equal(c.state.title, '删除项目');
    assert.equal(c.state.description, '确认要删除吗？');
    assert.equal(c.state.confirmLabel, '删除');
    assert.equal(c.state.destructive, true);

    c.onConfirm();
    assert.equal(await promise, true);
    assert.equal(c.state.open, false);
  });

  it('resolves false on cancel', async () => {
    const c = useConfirm();
    const promise = c.ask({
      title: '删除',
      description: 'desc',
    });

    c.onCancel();
    assert.equal(await promise, false);
    assert.equal(c.state.open, false);
  });

  it('treats onOpenChange(false) as cancel', async () => {
    const c = useConfirm();
    const promise = c.ask({ title: 't', description: 'd' });

    c.onOpenChange(false);
    assert.equal(await promise, false);
  });

  it('treats onOpenChange(true) as a no-op (keeps pending resolve)', async () => {
    const c = useConfirm();
    const promise = c.ask({ title: 't', description: 'd' });

    c.onOpenChange(true);
    c.onConfirm();
    assert.equal(await promise, true);
  });

  it('defaults confirmLabel/cancelLabel/destructive when not provided', () => {
    const c = useConfirm();
    void c.ask({ title: 't', description: 'd' });
    assert.equal(c.state.confirmLabel, '确认');
    assert.equal(c.state.cancelLabel, '取消');
    assert.equal(c.state.destructive, false);
  });
});

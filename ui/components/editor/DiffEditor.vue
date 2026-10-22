<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { MergeView } from '@codemirror/merge';
import { editorExtensions } from './extensions';

const original = defineModel<string>('original', { required: true });
const modified = defineModel<string>('modified', { required: true });
const container = ref<HTMLDivElement>();
const encoder = new TextEncoder();
const originalSize = computed(() => encoder.encode(original.value).length);
const modifiedSize = computed(() => encoder.encode(modified.value).length);
let view: MergeView | undefined;

onMounted(() => {
  if (!container.value) return;
  view = new MergeView({
    parent: container.value,
    a: {
      doc: original.value,
      extensions: editorExtensions({
        label: '原始文本',
        onChange: (value) => (original.value = value),
      }),
    },
    b: {
      doc: modified.value,
      extensions: editorExtensions({
        label: '修改后文本',
        onChange: (value) => (modified.value = value),
      }),
    },
    highlightChanges: true,
    gutter: true,
    diffConfig: { scanLimit: 500, timeout: 50 },
  });
});

watch([original, modified], ([a, b]) => {
  if (!view) return;
  for (const [editor, value] of [
    [view.a, a],
    [view.b, b],
  ] as const) {
    if (editor.state.doc.toString() !== value)
      editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: value } });
  }
});

onBeforeUnmount(() => view?.destroy());
</script>

<template>
  <section
    class="diff-editor overflow-hidden rounded-panel border border-solid border-border"
    aria-label="文本差异对比"
  >
    <header
      class="diff-editor-heading grid grid-cols-2 border-0 border-b border-solid border-border bg-surface text-[12px]"
    >
      <div><span>原始文本</span><span class="diff-removed">− 删除</span></div>
      <div><span>修改后文本</span><span class="diff-added">＋ 新增</span></div>
    </header>
    <div ref="container" class="diff-editor-content" />
    <footer
      class="diff-editor-footer grid grid-cols-2 border-0 border-t border-solid border-border text-[11px] text-muted"
    >
      <span>{{ originalSize.toLocaleString() }} 字节</span>
      <span>{{ modifiedSize.toLocaleString() }} 字节</span>
    </footer>
  </section>
</template>

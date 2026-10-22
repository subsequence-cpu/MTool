<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { json } from '@codemirror/lang-json';
import { editorExtensions } from './extensions';

const props = defineProps<{
  modelValue: string;
  label: string;
  readonly?: boolean;
  json?: boolean;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();
const container = ref<HTMLDivElement>();
const language = new Compartment();
let view: EditorView | undefined;

onMounted(() => {
  if (!container.value) return;
  view = new EditorView({
    parent: container.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        ...editorExtensions({
          label: props.label,
          readonly: props.readonly,
          onChange: (value) => emit('update:modelValue', value),
        }),
        language.of(props.json ? json() : []),
      ],
    }),
  });
});
watch(
  () => props.modelValue,
  (value) => {
    if (view && value !== view.state.doc.toString())
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
  },
);
watch(
  () => props.json,
  (value) => view?.dispatch({ effects: language.reconfigure(value ? json() : []) }),
);
onBeforeUnmount(() => view?.destroy());
</script>

<template><div ref="container" class="code-editor min-w-0 max-w-full overflow-hidden" /></template>

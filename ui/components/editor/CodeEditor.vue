<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { basicSetup } from 'codemirror';
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { json } from '@codemirror/lang-json';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';

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
        basicSetup,
        EditorView.lineWrapping,
        EditorState.readOnly.of(Boolean(props.readonly)),
        EditorView.editable.of(!props.readonly),
        EditorView.contentAttributes.of({ 'aria-label': props.label, spellcheck: 'false' }),
        language.of(props.json ? json() : []),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) emit('update:modelValue', update.state.doc.toString());
        }),
        syntaxHighlighting(
          HighlightStyle.define([
            { tag: tags.propertyName, color: '#8be9fd' },
            { tag: tags.string, color: '#f1fa8c' },
            { tag: [tags.number, tags.bool, tags.null], color: '#bd93f9' },
            { tag: tags.comment, color: '#adb6d1' },
          ]),
        ),
        EditorView.theme(
          {
            '&': {
              backgroundColor: 'var(--background)',
              color: 'var(--foreground)',
              fontSize: '13px',
            },
            '.cm-scroller': {
              fontFamily: 'var(--font-mono)',
              lineHeight: '1.9',
            },
            '.cm-content': {
              padding: '16px 0',
              minHeight: '310px',
              caretColor: 'var(--accent)',
            },
            '.cm-line': { padding: '0 16px' },
            '.cm-gutters': {
              backgroundColor: 'var(--background)',
              color: 'var(--muted)',
              border: 'none',
            },
            '.cm-lineNumbers .cm-gutterElement': { paddingLeft: '12px' },
            '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--surface)' },
            '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection': {
              backgroundColor: 'var(--selection)',
            },
            '&.cm-focused': { outline: 'none' },
            '.cm-cursor': { borderLeftColor: 'var(--accent)' },
          },
          { dark: true },
        ),
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

<template><div ref="container" class="code-editor" /></template>

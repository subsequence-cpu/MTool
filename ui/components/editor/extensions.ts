import { basicSetup } from 'codemirror';
import { Compartment, EditorState, type Extension } from '@codemirror/state';
import { EditorView, ViewPlugin } from '@codemirror/view';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { watch } from 'vue';
import { currentTheme } from '../../app/theme';

export function editorExtensions(options: {
  label: string;
  readonly?: boolean;
  onChange: (value: string) => void;
}): Extension[] {
  const appearance = new Compartment();
  const theme = () => EditorView.theme({}, { dark: currentTheme.value === 'dracula' });
  return [
    basicSetup,
    appearance.of(theme()),
    ViewPlugin.define((view) => {
      const stop = watch(currentTheme, () =>
        view.dispatch({ effects: appearance.reconfigure(theme()) }),
      );
      return { destroy: stop };
    }),
    EditorView.lineWrapping,
    EditorState.readOnly.of(Boolean(options.readonly)),
    EditorView.editable.of(!options.readonly),
    EditorView.contentAttributes.of({
      'aria-label': options.label,
      'aria-readonly': String(Boolean(options.readonly)),
      role: 'textbox',
      'aria-multiline': 'true',
      spellcheck: 'false',
      tabindex: '0',
    }),
    EditorView.dragMovesSelection.of(() => false),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) options.onChange(update.state.doc.toString());
    }),
    syntaxHighlighting(
      HighlightStyle.define([
        { tag: tags.propertyName, color: 'var(--syntax-property)' },
        { tag: tags.string, color: 'var(--syntax-string)' },
        { tag: [tags.number, tags.bool, tags.null], color: 'var(--syntax-literal)' },
        { tag: tags.comment, color: 'var(--syntax-comment)' },
      ]),
    ),
    EditorView.theme({
      '&': {
        backgroundColor: 'var(--background)',
        color: 'var(--foreground)',
        fontSize: '13px',
      },
      '.cm-scroller': {
        fontFamily: 'var(--font-mono)',
        lineHeight: '1.9',
        overflow: 'auto',
        overscrollBehavior: 'contain',
      },
      '.cm-content': {
        padding: '16px 0',
        minHeight: '310px',
        caretColor: 'var(--accent)',
        userSelect: 'text',
        WebkitUserSelect: 'text',
        cursor: 'text',
      },
      '.cm-line': { padding: '0 16px' },
      '.cm-gutters': {
        backgroundColor: 'var(--background)',
        color: 'var(--muted)',
        border: 'none',
      },
      '.cm-lineNumbers .cm-gutterElement': { paddingLeft: '12px' },
      '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'transparent' },
      '&.cm-focused .cm-activeLine, &.cm-focused .cm-activeLineGutter': {
        backgroundColor: 'var(--editor-active-line)',
      },
      '.cm-selectionBackground': {
        backgroundColor: 'var(--editor-selection-inactive)',
      },
      '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': {
        backgroundColor: 'var(--editor-selection)',
      },
      '&.cm-focused': { outline: 'none' },
      '.cm-cursor': { borderLeftColor: 'var(--accent)' },
    }),
  ];
}

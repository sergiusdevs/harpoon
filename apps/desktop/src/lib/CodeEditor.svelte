<script lang="ts">
  // Envoltorio mínimo de CodeMirror 6: JSON con plegado, o texto plano.
  import { onMount } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { json } from "@codemirror/lang-json";
  import { foldGutter, syntaxHighlighting, HighlightStyle, bracketMatching } from "@codemirror/language";
  import { tags } from "@lezer/highlight";

  interface Props {
    value: string;
    language?: "json" | "text";
    readonly?: boolean;
    onchange?: (value: string) => void;
    placeholder?: string;
  }
  let { value, language = "text", readonly = false, onchange, placeholder = "" }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView;
  const readonlyConf = new Compartment();
  const langConf = new Compartment();

  const highlight = HighlightStyle.define([
    { tag: tags.string, class: "tok-string" },
    { tag: tags.number, class: "tok-number" },
    { tag: tags.bool, class: "tok-bool" },
    { tag: tags.null, class: "tok-null" },
    { tag: tags.propertyName, class: "tok-propertyName" },
    { tag: tags.punctuation, class: "tok-punctuation" },
    { tag: tags.invalid, class: "tok-invalid" },
  ]);

  const langExt = (l: string) => (l === "json" ? [json(), foldGutter()] : []);

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          drawSelection(),
          bracketMatching(),
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          syntaxHighlighting(highlight),
          langConf.of(langExt(language)),
          readonlyConf.of([EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly)]),
          EditorView.updateListener.of((u) => {
            if (u.docChanged && onchange) onchange(u.state.doc.toString());
          }),
          EditorView.theme({ "&": { height: "100%" } }),
          EditorView.contentAttributes.of({ "aria-placeholder": placeholder }),
        ],
      }),
    });
    return () => view.destroy();
  });

  // Sincroniza cambios externos (p. ej. cambiar de solicitud) sin pisar la edición.
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (value !== current) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
    }
  });

  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: [
        langConf.reconfigure(langExt(language)),
        readonlyConf.reconfigure([EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly)]),
      ],
    });
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
</style>

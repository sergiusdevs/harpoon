<script lang="ts">
  // Envoltorio mínimo de CodeMirror 6: JSON con plegado, o texto plano.
  import { autocompletion, type CompletionContext } from "@codemirror/autocomplete";
  import { graphqlCompletions } from "../../domain/graphql";
  import { onMount } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { json } from "@codemirror/lang-json";
  import { javascript } from "@codemirror/lang-javascript";
  import { StreamLanguage, foldGutter, syntaxHighlighting, HighlightStyle, bracketMatching } from "@codemirror/language";
  import { search, searchKeymap, highlightSelectionMatches, openSearchPanel } from "@codemirror/search";
  import { tags } from "@lezer/highlight";

  interface Props {
    value: string;
    language?: "json" | "text" | "javascript" | "graphql";
    schema?: string;
    label?: string;
    readonly?: boolean;
    onchange?: (value: string) => void;
    placeholder?: string;
  }
  let { value, language = "text", readonly = false, onchange, placeholder = "", schema = "", label = "Editor de código" }: Props = $props();

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
    { tag: tags.keyword, class: "tok-keyword" },
    { tag: tags.comment, class: "tok-comment" },
    { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], class: "tok-fn" },
    { tag: tags.variableName, class: "tok-var" },
    { tag: tags.operator, class: "tok-op" },
  ]);

  const graphqlLanguage = StreamLanguage.define({
    token(stream) {
      if (stream.eatSpace()) return null;
      if (stream.match(/^#.*/)) return "comment";
      if (stream.match(/^"(?:[^"\\]|\\.)*"/)) return "string";
      if (stream.match(/^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/)) return "number";
      if (stream.match(/^\$[_A-Za-z][_0-9A-Za-z]*/)) return "variableName";
      if (stream.match(/^(?:query|mutation|subscription|fragment|on)\b/)) return "keyword";
      if (stream.match(/^(?:true|false|null)\b/)) return "bool";
      if (stream.match(/^@[_A-Za-z][_0-9A-Za-z]*/)) return "meta";
      if (stream.match(/^[_A-Za-z][_0-9A-Za-z]*/)) return "propertyName";
      stream.next(); return "punctuation";
    },
  });
  function completeGraphql(context: CompletionContext) {
    const word = context.matchBefore(/[_A-Za-z][_0-9A-Za-z]*/);
    if (!word && !context.explicit) return null;
    return { from: word?.from ?? context.pos, options: graphqlCompletions(schema, context.state.doc.toString(), context.pos), validFor: /^[_0-9A-Za-z]*$/ };
  }
  const langExt = (l: string) => l === "graphql" ? [graphqlLanguage, autocompletion({ override: [completeGraphql] })] : l === "json" ? [json(), foldGutter()] : l === "javascript" ? [javascript(), foldGutter()] : [];


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
          search({ top: true }),
          EditorState.phrases.of({
            Find: "Buscar",
            Replace: "Reemplazar",
            next: "siguiente",
            previous: "anterior",
            all: "todas",
            "match case": "mayúsculas",
            regexp: "regex",
            "by word": "palabra",
            replace: "reemplazar",
            "replace all": "reemplazar todo",
            close: "cerrar",
          }),
          highlightSelectionMatches(),
          keymap.of([...searchKeymap, ...defaultKeymap, ...historyKeymap]),
          syntaxHighlighting(highlight),
          langConf.of(langExt(language)),
          readonlyConf.of([EditorState.readOnly.of(readonly)]),
          EditorView.updateListener.of((u) => {
            if (u.docChanged && onchange) onchange(u.state.doc.toString());
          }),
          EditorView.theme({ "&": { height: "100%" } }),
          EditorView.contentAttributes.of({ "aria-placeholder": placeholder, "aria-label": label }),
        ],
      }),
    });
    return () => view.destroy();
  });

  /** Abre el panel de búsqueda (equivale a ⌘F con el editor enfocado). */
  export function openSearch() {
    view?.focus();
    if (view) openSearchPanel(view);
  }

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
        readonlyConf.reconfigure([EditorState.readOnly.of(readonly)]),
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

<script lang="ts">
  let { content = '', readonly = false, onchange }: {
    content: string;
    readonly?: boolean;
    onchange?: (value: string) => void;
  } = $props();

  let edited = $state('');

  $effect(() => { edited = content; });

  function handleInput(e: Event) {
    edited = (e.target as HTMLTextAreaElement).value;
    onchange?.(edited);
  }

  function handleKeydown(e: KeyboardEvent) {
    // Allow Tab to insert spaces instead of changing focus
    if (e.key === 'Tab') {
      e.preventDefault();
      const textarea = e.target as HTMLTextAreaElement;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const value = textarea.value;
      textarea.value = value.substring(0, start) + '  ' + value.substring(end);
      textarea.selectionStart = textarea.selectionEnd = start + 2;
      edited = textarea.value;
      onchange?.(edited);
    }
  }
</script>

<div class="yaml-editor">
  {#if edited !== content && !readonly}
    <div class="dirty-indicator">● Modified</div>
  {/if}
  <textarea
    value={edited}
    oninput={handleInput}
    onkeydown={handleKeydown}
    {readonly}
    spellcheck="false"
    class="editor"
    aria-label="YAML editor"
  ></textarea>
</div>

<style>
  .yaml-editor { position: relative; height: 100%; }
  .editor {
    width: 100%;
    height: calc(100vh - 320px);
    min-height: 300px;
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.75rem;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 0.8rem;
    line-height: 1.5;
    resize: vertical;
    tab-size: 2;
    white-space: pre;
    overflow: auto;
  }
  .editor:focus { border-color: #58a6ff; outline: none; }
  .editor[readonly] { opacity: 0.8; cursor: default; }
  .dirty-indicator {
    position: absolute;
    top: 0.5rem;
    right: 0.75rem;
    color: #d29922;
    font-size: 0.75rem;
    z-index: 1;
  }
</style>

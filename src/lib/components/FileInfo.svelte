<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { humanSize, extOf } from "$lib/api";
  import { convertFileSrc } from "@tauri-apps/api/core";

  const item = $derived(session.current);

  // Dimensions/duration are read natively from the loaded media element.
  let dims = $state<string>("—");
  let duration = $state<string>("—");

  $effect(() => {
    const it = item;
    dims = "—";
    duration = "—";
    if (!it) return;
    const url = convertFileSrc(it.path);
    if (it.kind === "image") {
      const img = new Image();
      img.onload = () => (dims = `${img.naturalWidth} × ${img.naturalHeight}`);
      img.src = url;
    } else if (it.kind === "video") {
      const v = document.createElement("video");
      v.preload = "metadata";
      v.onloadedmetadata = () => {
        dims = `${v.videoWidth} × ${v.videoHeight}`;
        duration = fmtDuration(v.duration);
      };
      v.src = url;
    }
  });

  function fmtDuration(s: number): string {
    if (!isFinite(s)) return "—";
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, "0")}`;
  }
  function fmtDate(ms: number | null): string {
    if (!ms) return "—";
    return new Date(ms).toLocaleString();
  }
</script>

<section class="pane">
  <div class="title">「 File Info 」</div>
  {#if item}
    <dl>
      <dt>Name</dt><dd title={item.fileName}>{item.fileName}</dd>
      <dt>Size</dt><dd>{humanSize(item.sizeBytes)}</dd>
      <dt>Dimensions</dt><dd>{dims}</dd>
      {#if item.kind === "video"}<dt>Duration</dt><dd>{duration}</dd>{/if}
      <dt>Type</dt><dd>{item.kind}</dd>
      <dt>Ext</dt><dd>{extOf(item.fileName).toUpperCase() || "—"}</dd>
      <dt>Modified</dt><dd>{fmtDate(item.modifiedMs)}</dd>
    </dl>
  {:else}
    <div class="empty">—</div>
  {/if}
</section>

<style>
  .pane {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 0;
    overflow: hidden;
  }
  .title { padding: 2px 12px 8px; color: var(--purple); font-weight: 600; }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 12px;
    margin: 0;
    padding: 0 12px;
    font-size: 12px;
  }
  dt { color: var(--text-muted); }
  dd {
    margin: 0;
    color: var(--text-primary);
    text-align: right;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-variant-numeric: tabular-nums;
  }
  .empty { padding: 4px 12px; color: var(--text-muted); }
</style>

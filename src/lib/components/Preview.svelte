<script lang="ts">
  import { session } from "$lib/session.svelte";
  import { settings } from "$lib/settings.svelte";
  import { humanSize, extOf } from "$lib/api";
  import { kindIcon } from "$lib/icons";
  import { convertFileSrc } from "@tauri-apps/api/core";

  const item = $derived(session.current);
  const src = $derived(item ? convertFileSrc(item.path) : "");
  // Re-key the media element on path change so the browser reloads cleanly.
  const ext = $derived(item ? extOf(item.fileName) : "");
</script>

<section class="pane">
  <div class="bar">
    <span class="name" title={item?.fileName ?? ""}>
      {#if item}<span class="nf kind">{kindIcon(item.kind)}</span>{/if}{item?.fileName ?? "—"}
    </span>
    {#if item}
      <span class="meta">
        <span class="chip ext-{ext || 'other'}">{ext.toUpperCase() || "?"}</span>
        <span class="size">{humanSize(item.sizeBytes)}</span>
        <span class="pos">{session.cursor + 1}/{session.total}</span>
      </span>
    {/if}
  </div>

  <div class="stage">
    {#if !item}
      <div class="placeholder">Nothing to preview</div>
    {:else if item.kind === "image"}
      {#key item.path}
        <img {src} alt={item.fileName} />
      {/key}
    {:else if item.kind === "video"}
      {#key item.path}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video {src} controls autoplay={settings.videoAutoplay} muted={settings.videoMuted} loop={settings.videoLoop}></video>
      {/key}
    {:else}
      <div class="placeholder">No preview for this file type</div>
    {/if}
  </div>
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-muted);
  }
  .name {
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name .kind { margin-right: 7px; font-size: 12px; opacity: 0.8; color: var(--purple); }
  .meta {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .size { color: var(--text-secondary); font-variant-numeric: tabular-nums; }
  .pos { color: var(--text-muted); font-family: var(--mono); font-size: 11px; }
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background:
      radial-gradient(circle at 50% 40%, #161b22 0%, var(--bg-app) 80%);
  }
  img, video {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-sm);
    box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5);
  }
  .placeholder {
    color: var(--text-muted);
    font-size: 14px;
  }
</style>

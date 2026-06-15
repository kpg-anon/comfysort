<script lang="ts">
  // Shared themed confirmation modal (cross-drive move, folder delete, recursion).
  // Interaction pattern: split choice cards. Each option is a large clickable
  // card (icon + title + description); the recommended one carries an accent
  // border + check badge and is the keyboard default. A quiet Cancel link sits
  // below. Presentational — the caller owns state; keyboard handling stays in
  // +page.svelte's dispatcher.
  import type { Snippet } from "svelte";
  import { I } from "$lib/icons";

  /** One choice card. The recommended card is the keyboard default (Enter/y). */
  type Choice = {
    icon: string;
    title: string;
    desc: string;
    /** Accent border + check badge; this is the highlighted default. */
    recommended?: boolean;
    /** Per-card highlight color, overriding the modal accent — e.g. a "keep"
     *  card in a destructive dialog should hover purple, not red. */
    accent?: "orange" | "red" | "green" | "purple" | "cyan";
    /** Shortcut key shown in the card corner (also wired in +page.svelte). */
    key?: string;
    action: () => void;
  };

  let {
    accent = "orange",
    icon,
    title,
    subtitle,
    choices,
    cancel,
    children,
  }: {
    /** Theme color var name driving the header glyph + recommended card. */
    accent?: "orange" | "red" | "green" | "purple" | "cyan";
    icon: string;
    title: string;
    subtitle: string;
    choices: Choice[];
    /** The quiet dismiss link below the cards. */
    cancel?: { key: string; label: string; action: () => void };
    children: Snippet;
  } = $props();
</script>

<div class="scrim">
  <div class="modal" style="--accent: var(--{accent})">
    <div class="mhead">
      <span class="micon nf">{icon}</span>
      <div class="mheadtext">
        <div class="mtitle">{title}</div>
        <div class="msub">{subtitle}</div>
      </div>
    </div>
    <div class="mbody">{@render children()}</div>
    <div class="choices" style="grid-template-columns: repeat({choices.length}, 1fr)">
      {#each choices as c (c.title)}
        <button
          class="choice"
          class:recommended={c.recommended}
          style={c.accent ? `--card-accent: var(--${c.accent})` : undefined}
          onclick={c.action}
        >
          {#if c.recommended}<span class="check nf">{I.check}</span>{/if}
          <span class="cicon nf">{c.icon}</span>
          <span class="ctitle">{c.title}</span>
          <span class="cdesc">{c.desc}</span>
        </button>
      {/each}
    </div>
    {#if cancel}
      <button class="mcancel" onclick={cancel.action}>{cancel.label}</button>
    {/if}
  </div>
</div>

<style>
  /* Flat scrim matching the Settings overlay — no blur, just a dim. */
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 13, 0.62);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: mfade 0.12s ease-out;
  }
  /* Flat panel: same surface, border, and radius language as every other pane.
     No accent glow, no gradient — the accent only colors the header glyph and
     tints the primary action. */
  .modal {
    width: 420px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 18px 18px 16px;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.4),
      0 12px 32px rgba(0, 0, 0, 0.36);
    animation: mpop 0.13s ease-out;
  }
  @keyframes mfade { from { opacity: 0; } }
  @keyframes mpop { from { opacity: 0; transform: translateY(6px) scale(0.99); } }

  .mhead { display: flex; gap: 11px; align-items: center; margin-bottom: 12px; }
  /* Flat accent chip — same shape as the pane header buttons, faint accent
     border to tie in the accent color without a glow. */
  .micon {
    flex: none; width: 30px; height: 30px; border-radius: var(--radius-sm);
    display: grid; place-items: center; font-size: 14px;
    color: var(--accent);
    background: var(--bg-chip);
    border: 1px solid color-mix(in srgb, var(--accent) 32%, var(--border));
  }
  .mtitle { color: var(--text-primary); font-weight: 600; font-size: 14px; }
  .msub { color: var(--text-muted); font-size: 11px; margin-top: 1px; line-height: 1.45; }
  .mbody { color: var(--text-secondary); font-size: 12.5px; margin: 0 0 14px; line-height: 1.5; text-align: center; }
  .mbody :global(b) { color: var(--text-primary); }

  /* Split choice cards — centered content with a large icon, mirroring the
     reference. */
  .choices { display: grid; gap: 9px; }
  .choice {
    /* Per-card highlight color; defaults to the modal accent, overridable via an
       inline --card-accent (e.g. a "keep" card hovers purple in a red dialog). */
    --card-accent: var(--accent);
    position: relative;
    display: flex; flex-direction: column; align-items: center; gap: 5px;
    padding: 20px 14px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-chip);
    cursor: pointer; text-align: center;
    transition: border-color 0.12s, background 0.12s, transform 0.07s ease;
  }
  /* Colored highlight on hover — accent border + faint accent wash + accent icon. */
  .choice:hover {
    border-color: var(--card-accent);
    background: color-mix(in srgb, var(--card-accent) 8%, var(--bg-chip));
  }
  /* Tactile press response. */
  .choice:active { transform: scale(0.95); }
  /* Recommended (default) card: accent border + faint accent tint + check badge. */
  .choice.recommended {
    border-color: var(--card-accent);
    background: color-mix(in srgb, var(--card-accent) 12%, var(--bg-chip));
  }
  .choice.recommended:hover { background: color-mix(in srgb, var(--card-accent) 18%, var(--bg-chip)); }
  .cicon { font-size: 42px; line-height: 1; color: var(--text-muted); margin-bottom: 9px; }
  .choice:hover .cicon { color: var(--card-accent); }
  .choice.recommended .cicon { color: var(--card-accent); }
  .ctitle { color: var(--text-primary); font-size: 13px; font-weight: 600; }
  .cdesc { color: var(--text-muted); font-size: 11px; line-height: 1.4; }
  /* Selected badge (filled accent circle, white check) — mirrors the reference. */
  .check {
    position: absolute; top: 9px; right: 9px;
    width: 18px; height: 18px; border-radius: 50%;
    display: grid; place-items: center; font-size: 9px;
    background: var(--card-accent); color: var(--text-inverse);
  }
  /* Dismiss button below the cards — a flat chip in the same language as the
     rest of the interface (border + bg-chip), full width under the cards. */
  .mcancel {
    display: block; width: 100%; margin-top: 10px; padding: 8px;
    background: var(--bg-chip); border: 1px solid var(--border);
    border-radius: var(--radius-sm); cursor: pointer;
    color: var(--text-secondary); font-size: 12px;
    transition: border-color 0.12s, color 0.12s, background 0.12s, transform 0.07s ease;
  }
  /* Red highlight on hover — the cancel/dismiss reads as the "back out" action. */
  .mcancel:hover {
    border-color: var(--red);
    color: var(--text-primary);
    background: color-mix(in srgb, var(--red) 8%, var(--bg-chip));
  }
  .mcancel:active { transform: scale(0.96); }
</style>

<script lang="ts">
  // Shared themed confirmation modal (cross-drive move, folder delete, …).
  // Presentational: the caller owns the state and supplies the body snippet
  // plus a button row; keyboard handling stays in +page.svelte's dispatcher.
  import type { Snippet } from "svelte";

  type Btn = {
    key: string;
    label: string;
    /** primary = solid accent · accent = outlined accent text · ghost = muted */
    kind?: "primary" | "accent" | "ghost";
    action: () => void;
  };

  let {
    accent = "orange",
    icon,
    title,
    subtitle,
    buttons,
    children,
  }: {
    /** Theme color var name driving the header icon, top bar, and primary button. */
    accent?: "orange" | "red" | "green" | "purple" | "cyan";
    icon: string;
    title: string;
    subtitle: string;
    buttons: Btn[];
    children: Snippet;
  } = $props();
</script>

<div class="scrim">
  <div class="modal" style="--accent: var(--{accent})">
    <div class="glow" aria-hidden="true"></div>
    <div class="mhead">
      <span class="micon nf">{icon}</span>
      <div class="mheadtext">
        <div class="mtitle">{title}</div>
        <div class="msub">{subtitle}</div>
      </div>
    </div>
    <div class="mbody">{@render children()}</div>
    <div class="mrow">
      {#each buttons as b (b.key)}
        <button class="mbtn {b.kind ?? 'ghost'}" onclick={b.action}>
          <kbd>{b.key}</kbd>
          {b.label}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 13, 0.58);
    backdrop-filter: blur(3px);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: mfade 0.14s ease-out;
  }
  .modal {
    position: relative;
    width: 480px;
    background: var(--bg-panel);
    border: 1px solid color-mix(in srgb, var(--accent) 28%, var(--border));
    border-radius: 14px;
    padding: 20px 22px 18px;
    box-shadow:
      0 28px 80px rgba(0, 0, 0, 0.6),
      0 0 44px color-mix(in srgb, var(--accent) 14%, transparent);
    overflow: hidden;
    animation: mpop 0.16s cubic-bezier(0.2, 1.1, 0.4, 1);
  }
  /* soft accent wash behind the header so the modal reads as a warning at a
     glance without a hard colored bar */
  .glow {
    position: absolute;
    inset: -1px -1px auto;
    height: 86px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--accent) 13%, transparent),
      transparent
    );
    border-top: 2px solid var(--accent);
    pointer-events: none;
  }
  @keyframes mfade { from { opacity: 0; } }
  @keyframes mpop { from { opacity: 0; transform: translateY(10px) scale(0.97); } }
  .mhead { position: relative; display: flex; gap: 13px; align-items: flex-start; margin-bottom: 14px; }
  .micon {
    flex: none; width: 38px; height: 38px; border-radius: 11px;
    display: grid; place-items: center; font-size: 17px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--border));
    box-shadow: 0 0 18px color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .mtitle { color: var(--text-primary); font-weight: 700; font-size: 14.5px; }
  .msub { color: var(--text-muted); font-size: 11.5px; margin-top: 2px; line-height: 1.45; }
  .mbody { position: relative; color: var(--text-secondary); font-size: 13px; margin: 0 0 16px; line-height: 1.55; }
  .mbody :global(b) { color: var(--text-primary); }
  .mrow { display: flex; gap: 8px; }
  .mbtn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 9px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-chip);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    transition: border-color 0.1s, color 0.1s, filter 0.1s;
  }
  .mbtn:hover { border-color: var(--text-muted); color: var(--text-primary); }
  .mbtn.primary {
    background: var(--accent); color: var(--text-inverse);
    border-color: var(--accent); font-weight: 600;
  }
  .mbtn.primary:hover { filter: brightness(1.08); border-color: var(--accent); }
  .mbtn.accent { color: var(--accent); }
  .mbtn.accent:hover { border-color: var(--accent); }
  .mbtn.ghost { color: var(--text-muted); }
  .mbtn kbd {
    font-family: var(--mono);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
    font-size: 11px;
  }
  .mbtn.primary kbd {
    background: rgba(0, 0, 0, 0.2);
    border-color: rgba(0, 0, 0, 0.24);
    color: var(--text-inverse);
  }
</style>

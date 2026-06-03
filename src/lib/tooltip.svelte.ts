// Shared themed tooltip. A `use:tip` action drives one fixed-position popover
// (rendered once by Tooltip.svelte at the app root), so the tooltip is never
// clipped by a pane's `overflow: hidden` — unlike a CSS `::after` tooltip.

class TooltipState {
  text = $state("");
  x = $state(0);
  y = $state(0);
  visible = $state(false);

  show(text: string, x: number, y: number) {
    this.text = text;
    this.x = x;
    this.y = y;
    this.visible = true;
  }
  hide() {
    this.visible = false;
  }
}

export const tooltip = new TooltipState();

/** Attach a themed tooltip to an element: `use:tip={"label or path"}`. */
export function tip(node: HTMLElement, text: string) {
  let current = text;
  const place = () => {
    const r = node.getBoundingClientRect();
    // Clamp so a long path/list never runs off the right or left edge.
    const x = Math.max(8, Math.min(r.left, window.innerWidth - 468));
    tooltip.show(current, x, r.bottom + 7);
  };
  const enter = () => {
    if (current) place();
  };
  const leave = () => tooltip.hide();
  node.addEventListener("mouseenter", enter);
  node.addEventListener("mouseleave", leave);
  node.addEventListener("mousedown", leave); // dismiss on click
  return {
    update(t: string) {
      current = t;
      if (tooltip.visible) place();
    },
    destroy() {
      node.removeEventListener("mouseenter", enter);
      node.removeEventListener("mouseleave", leave);
      node.removeEventListener("mousedown", leave);
      leave();
    },
  };
}

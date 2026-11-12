export interface PaneIntent {
  open: boolean;
  preferredSize: number;
  minSize: number;
  maxSize?: number;
  collapsedSize?: number;
}

export interface PaneLayout {
  open: boolean;
  size: number;
  min: number;
  max: number;
}

export function clampSize(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/** Panes are ordered from highest to lowest priority. Space for collapsed
 * controls and the main region precedes expanded panes and preferred sizes. */
export function fitPanes(
  availableSize: number,
  mainMinimum: number,
  intents: readonly PaneIntent[],
): { panes: PaneLayout[]; mainSize: number } {
  let free = Math.max(0, availableSize);
  const panes: PaneLayout[] = intents.map((intent) => {
    const size = Math.min(intent.collapsedSize ?? 0, free);
    free -= size;
    return { open: false, size, min: size, max: size };
  });
  free -= mainMinimum;

  for (const [index, intent] of intents.entries()) {
    const pane = panes[index]!;
    const min = Math.max(pane.size, intent.minSize);
    const expansion = min - pane.size;
    if (!intent.open || expansion > free) continue;
    free -= expansion;
    pane.open = true;
    pane.size = min;
    pane.min = min;
  }

  for (const [index, intent] of intents.entries()) {
    const pane = panes[index]!;
    if (!pane.open) continue;
    const max = Math.max(pane.min, intent.maxSize ?? availableSize);
    const size = clampSize(intent.preferredSize, pane.min, Math.min(max, pane.size + free));
    free -= size - pane.size;
    pane.size = size;
  }

  for (const [index, intent] of intents.entries()) {
    const pane = panes[index]!;
    if (pane.open) pane.max = Math.max(pane.min, Math.min(intent.maxSize ?? availableSize, pane.size + free));
  }
  return { panes, mainSize: Math.max(0, mainMinimum + free) };
}

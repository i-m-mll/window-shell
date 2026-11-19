export type ShellHost = 'native' | 'browser';

export interface TitleBarActions {
  startDrag: () => void;
  toggleZoom: () => void;
}

const DRAG_REGION = '[data-shell-drag-region]';
const INTERACTIVE = 'a,button,input,textarea,select,option,summary,label,[role="button"],[role="link"],[role="menuitem"],[role="tab"],[role="slider"],[role="separator"],[tabindex]:not([tabindex="-1"]),[contenteditable]:not([contenteditable="false"]),[data-shell-drag-exclude]';

/** The native host sets `data-shell-host="native"` before the page runs; a page without it is in a browser. */
export function shellHost(doc: Document = document): ShellHost {
  return doc.documentElement.dataset.shellHost === 'native' ? 'native' : 'browser';
}

function dragRegion(event: MouseEvent): Element | null {
  const target = event.target instanceof Element ? event.target : null;
  const region = target?.closest(DRAG_REGION);
  if (!target || !region) return null;
  const interactive = target.closest(INTERACTIVE);
  return interactive && region.contains(interactive) ? null : region;
}

/** A press on a drag region outside its interactive elements drags the window; a double click there zooms it. */
export function installTitleBarDrag(root: Document | HTMLElement, actions: TitleBarActions): () => void {
  const controller = new AbortController();
  const listener = { signal: controller.signal };
  let press: { region: Element; x: number; y: number } | undefined;

  const down = (event: MouseEvent) => {
    press = undefined;
    if (event.defaultPrevented || event.button !== 0) return;
    const region = dragRegion(event);
    if (!region) return;
    event.preventDefault();
    if (event.detail === 1) actions.startDrag();
    else if (event.detail === 2) press = { region, x: event.clientX, y: event.clientY };
  };
  const up = (event: MouseEvent) => {
    const start = press;
    press = undefined;
    if (start && event.detail === 2 && dragRegion(event) === start.region &&
        event.clientX === start.x && event.clientY === start.y) actions.toggleZoom();
  };
  root.addEventListener('mousedown', down as EventListener, listener);
  root.addEventListener('mouseup', up as EventListener, listener);
  return () => controller.abort();
}

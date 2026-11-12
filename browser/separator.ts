import { clampSize, type PaneLayout } from './layout';

type Axis = 'x' | 'y';

interface SeparatorOptions {
  axis: Axis;
  direction: 1 | -1;
  layout: () => PaneLayout;
  onPreview: (size: number | null) => void;
  onResize: (size: number) => void;
}

const STEP = 8;
const LARGE_STEP = 32;

export function separatorValues(layout: PaneLayout, axis: Axis) {
  return {
    'aria-orientation': axis === 'x' ? 'vertical' as const : 'horizontal' as const,
    'aria-valuemin': layout.min,
    'aria-valuemax': layout.max,
    'aria-valuenow': layout.size,
  };
}

export function installSeparator(element: HTMLElement, options: SeparatorOptions): () => void {
  const { axis, direction } = options;
  const coordinate = axis === 'x' ? (event: PointerEvent) => event.clientX : (event: PointerEvent) => event.clientY;
  const [negativeKey, positiveKey] = axis === 'x' ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
  const controller = new AbortController();
  const listener = { signal: controller.signal };
  let drag: { id: number; coordinate: number; size: number } | undefined;

  const release = () => {
    const current = drag;
    drag = undefined;
    if (current && element.hasPointerCapture(current.id)) element.releasePointerCapture(current.id);
    options.onPreview(null);
  };
  const pointerSize = (event: PointerEvent): number | null => {
    const pane = options.layout();
    if (!drag || !pane.open) return null;
    const delta = direction * (coordinate(event) - drag.coordinate);
    const size = clampSize(drag.size + delta, pane.min, pane.max);
    return delta && size !== drag.size ? size : null;
  };
  element.addEventListener('pointerdown', (event) => {
    const pane = options.layout();
    if (event.button !== 0 || drag || !pane.open || pane.min === pane.max) return;
    event.preventDefault();
    element.focus({ preventScroll: true });
    element.setPointerCapture(event.pointerId);
    drag = { id: event.pointerId, coordinate: coordinate(event), size: pane.size };
  }, listener);
  element.addEventListener('pointermove', (event) => {
    if (drag?.id !== event.pointerId) return;
    options.onPreview(pointerSize(event));
  }, listener);
  element.addEventListener('pointerup', (event) => {
    if (drag?.id !== event.pointerId) return;
    const size = pointerSize(event);
    release();
    if (size !== null) options.onResize(size);
  }, listener);
  const cancelDrag = (event: PointerEvent) => {
    if (drag?.id === event.pointerId) release();
  };
  element.addEventListener('pointercancel', cancelDrag, listener);
  element.addEventListener('lostpointercapture', cancelDrag, listener);
  element.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && drag) {
      event.preventDefault();
      event.stopPropagation();
      release();
      return;
    }
    const pane = options.layout();
    if (!pane.open) return;
    const step = event.shiftKey ? LARGE_STEP : STEP;
    let size: number;
    switch (event.key) {
      case negativeKey: size = pane.size - direction * step; break;
      case positiveKey: size = pane.size + direction * step; break;
      case 'Home': size = pane.min; break;
      case 'End': size = pane.max; break;
      default: return;
    }
    event.preventDefault();
    const next = clampSize(size, pane.min, pane.max);
    if (next !== pane.size) options.onResize(next);
  }, listener);
  return () => {
    controller.abort();
    if (drag) release();
  };
}

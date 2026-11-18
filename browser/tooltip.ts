const DELAY_MS = 120;
const EDGE_INSET = 8;
const TOOLTIP_ID = 'app-hover-tip';

export function appTip(label: string | undefined, placement: 'bottom' | 'right' = 'bottom') {
  return { 'data-app-tip': label, 'data-app-tip-placement': placement };
}

function positionTip(node: HTMLElement, anchor: HTMLElement, width: number, height: number): void {
  const target = anchor.getBoundingClientRect();
  const bounds = node.getBoundingClientRect();
  const right = anchor.dataset.appTipPlacement === 'right';
  const gap = EDGE_INSET;
  let left = right ? target.right + gap : target.left + (target.width - bounds.width) / 2;
  let top = right ? target.top + (target.height - bounds.height) / 2 : target.bottom + gap;
  if (right && left + bounds.width > width - gap) left = target.left - bounds.width - gap;
  if (!right && top + bounds.height > height - gap) top = target.top - bounds.height - gap;
  node.style.left = `${Math.max(gap, Math.min(left, width - bounds.width - gap))}px`;
  node.style.top = `${Math.max(gap, Math.min(top, height - bounds.height - gap))}px`;
}

export function installTooltips(doc: Document = document): () => void {
  const view = doc.defaultView!;
  const controller = new AbortController();
  const listener = { signal: controller.signal };
  const node = doc.createElement('div');
  node.id = TOOLTIP_ID;
  node.className = 'app-hover-tip';
  node.setAttribute('role', 'tooltip');
  let pending: number | undefined;
  let anchor: HTMLElement | null = null;

  const descriptionIds = (element: HTMLElement) =>
    (element.getAttribute('aria-describedby') ?? '').split(/\s+/).filter(Boolean);
  const hide = () => {
    view.clearTimeout(pending);
    pending = undefined;
    observer.disconnect();
    if (anchor && node.isConnected) {
      const remaining = descriptionIds(anchor).filter(id => id !== TOOLTIP_ID);
      if (remaining.length) anchor.setAttribute('aria-describedby', remaining.join(' '));
      else anchor.removeAttribute('aria-describedby');
    }
    node.remove();
    anchor = null;
  };
  const observer = new MutationObserver(() => {
    if (anchor && (!anchor.isConnected || !anchor.dataset.appTip)) hide();
  });
  const enter = (event: Event) => {
    const next = event.target instanceof Element ? event.target.closest<HTMLElement>('[data-app-tip]') : null;
    if (next === anchor) return;
    hide();
    if (!next?.dataset.appTip) return;
    anchor = next;
    pending = view.setTimeout(() => {
      if (!next.isConnected || !next.dataset.appTip) {
        hide();
        return;
      }
      node.textContent = next.dataset.appTip;
      doc.body.append(node);
      positionTip(node, next, view.innerWidth, view.innerHeight);
      next.setAttribute('aria-describedby', [...new Set([...descriptionIds(next), TOOLTIP_ID])].join(' '));
      observer.observe(doc.body, { childList: true, subtree: true });
      observer.observe(next, { attributes: true, attributeFilter: ['data-app-tip'] });
    }, DELAY_MS);
  };
  const leave = (event: PointerEvent | FocusEvent) => {
    if (!(event.relatedTarget instanceof Node) || !anchor?.contains(event.relatedTarget)) hide();
  };
  const dismiss = { ...listener, capture: true };
  doc.addEventListener('pointerover', enter, listener);
  doc.addEventListener('pointerout', leave, listener);
  doc.addEventListener('focusin', enter, listener);
  doc.addEventListener('focusout', leave, listener);
  doc.addEventListener('pointerdown', hide, dismiss);
  doc.addEventListener('click', hide, dismiss);
  doc.addEventListener('scroll', hide, dismiss);
  doc.addEventListener('keydown', event => { if (event.key === 'Escape') hide(); }, listener);
  view.addEventListener('resize', hide, listener);
  view.addEventListener('blur', hide, listener);
  return () => {
    controller.abort();
    hide();
  };
}

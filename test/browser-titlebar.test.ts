import { GlobalRegistrator } from '@happy-dom/global-registrator';
import { afterEach, beforeEach, expect, test } from 'bun:test';

GlobalRegistrator.register();
const { installTitleBarDrag } = await import('../browser/titlebar');

let calls: string[] = [];
let dispose = () => {};

beforeEach(() => {
  calls = [];
  document.body.innerHTML = `
    <header data-shell-drag-region tabindex="-1">
      <span id="logo"><svg id="glyph"></svg></span>
      <button id="button"><svg id="icon"></svg></button>
      <span id="label">Project</span>
    </header>
    <main id="content"></main>`;
  dispose = installTitleBarDrag(document, {
    startDrag: () => calls.push('drag'),
    toggleZoom: () => calls.push('zoom'),
  });
});

afterEach(() => dispose());

function press(id: string, detail: number, x = 10) {
  const target = document.getElementById(id)!;
  const init = { bubbles: true, cancelable: true, button: 0, detail, clientX: x, clientY: 5 };
  target.dispatchEvent(new MouseEvent('mousedown', init));
  target.dispatchEvent(new MouseEvent('mouseup', init));
}

test('a press on the region drags the window, including on its decorative children', () => {
  press('label', 1);
  press('glyph', 1);
  expect(calls).toEqual(['drag', 'drag']);
});

test('a press on a button inside the region, or outside the region, does nothing', () => {
  press('icon', 1);
  press('content', 1);
  press('icon', 2);
  expect(calls).toEqual([]);
});

test('a double click on the region zooms once and does not drag a second time', () => {
  press('label', 1);
  press('label', 2);
  expect(calls).toEqual(['drag', 'zoom']);
});

test('a second press that moves before release does not zoom', () => {
  const label = document.getElementById('label')!;
  const init = { bubbles: true, cancelable: true, button: 0, detail: 2, clientY: 5 };
  label.dispatchEvent(new MouseEvent('mousedown', { ...init, clientX: 10 }));
  label.dispatchEvent(new MouseEvent('mouseup', { ...init, clientX: 30 }));
  expect(calls).toEqual([]);
});

import { expect, test } from 'bun:test';
import { fitPanes } from '../browser/layout';

test('folds lower-priority drawers and restores their preferred sizes', () => {
  const left = { open: true, preferredSize: 240, minSize: 180 };
  const right = { open: true, preferredSize: 320, minSize: 280 };
  const panes = [left, right];
  expect(fitPanes(900, 240, panes)).toEqual({
    mainSize: 340,
    panes: [
      { open: true, size: 240, min: 180, max: 340 },
      { open: true, size: 320, min: 280, max: 420 },
    ],
  });
  expect(fitPanes(570, 240, panes)).toEqual({
    mainSize: 330,
    panes: [
      { open: true, size: 240, min: 180, max: 330 },
      { open: false, size: 0, min: 0, max: 0 },
    ],
  });
  expect(fitPanes(900, 240, panes).panes.map(pane => pane.size)).toEqual([240, 320]);
  expect(fitPanes(900, 240, [left, { ...right, open: false }]).panes[1]?.open).toBe(false);
});

test('retains collapsed shelf controls and bounds the split', () => {
  const bottom = { open: true, preferredSize: 320, minSize: 80, collapsedSize: 44 };
  expect(fitPanes(180, 80, [bottom])).toEqual({
    mainSize: 80, panes: [{ open: true, size: 100, min: 80, max: 100 }],
  });
  expect(fitPanes(130, 80, [bottom])).toEqual({
    mainSize: 86, panes: [{ open: false, size: 44, min: 44, max: 44 }],
  });
  expect(fitPanes(400, 80, [bottom]).panes[0]?.size).toBe(320);
  expect(fitPanes(10, 80, [bottom])).toEqual({
    mainSize: 0, panes: [{ open: false, size: 10, min: 10, max: 10 }],
  });
});

test('reserves both expanded minimums before honoring a preferred size', () => {
  const panes = [
    { open: true, preferredSize: 300, minSize: 150, maxSize: 300 },
    { open: true, preferredSize: 300, minSize: 150, maxSize: 300 },
  ];
  expect(fitPanes(600, 220, panes).panes).toEqual([
    { open: true, size: 230, min: 150, max: 230 },
    { open: true, size: 150, min: 150, max: 150 },
  ]);
});

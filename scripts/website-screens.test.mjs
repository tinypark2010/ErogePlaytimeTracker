import { describe, expect, it, vi } from 'vitest';
import { activateScreen } from '../website/screens.js';

function tab(screenTarget) {
  return {
    dataset: { screenTarget },
    attributes: {},
    focus: vi.fn(),
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
  };
}

describe('website screen selector', () => {
  it('shows the selected screen and hides the other panels', () => {
    const tabs = [tab('library'), tab('detail'), tab('statistics')];
    const panels = [
      { dataset: { screenPanel: 'library' }, hidden: false },
      { dataset: { screenPanel: 'detail' }, hidden: true },
      { dataset: { screenPanel: 'statistics' }, hidden: true },
    ];

    activateScreen(tabs, panels, tabs[1], true);

    expect(tabs.map((item) => item.attributes['aria-selected'])).toEqual([
      'false',
      'true',
      'false',
    ]);
    expect(tabs.map((item) => item.tabIndex)).toEqual([-1, 0, -1]);
    expect(panels.map((panel) => panel.hidden)).toEqual([true, false, true]);
    expect(tabs[1].focus).toHaveBeenCalledOnce();
  });
});

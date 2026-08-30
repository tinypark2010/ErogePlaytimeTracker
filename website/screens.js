export function activateScreen(tabs, panels, tab, moveFocus = false) {
  const target = tab.dataset.screenTarget;

  tabs.forEach((item) => {
    const selected = item === tab;
    item.setAttribute('aria-selected', String(selected));
    item.tabIndex = selected ? 0 : -1;
  });

  panels.forEach((panel) => {
    panel.hidden = panel.dataset.screenPanel !== target;
  });

  if (moveFocus) tab.focus();
}

if (typeof document !== 'undefined') {
  const tabs = Array.from(document.querySelectorAll('[data-screen-target]'));
  const panels = Array.from(document.querySelectorAll('[data-screen-panel]'));

  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => activateScreen(tabs, panels, tab));
    tab.addEventListener('keydown', (event) => {
      let nextIndex;

      if (event.key === 'ArrowDown' || event.key === 'ArrowRight') nextIndex = index + 1;
      if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') nextIndex = index - 1;
      if (event.key === 'Home') nextIndex = 0;
      if (event.key === 'End') nextIndex = tabs.length - 1;
      if (nextIndex === undefined) return;

      event.preventDefault();
      activateScreen(tabs, panels, tabs[(nextIndex + tabs.length) % tabs.length], true);
    });
  });
}

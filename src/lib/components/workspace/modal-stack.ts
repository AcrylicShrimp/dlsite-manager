// Notifications must belong to the active native dialog to remain visible and interactive.
const dialogs: HTMLDialogElement[] = [];
const listeners = new Set<() => void>();
export function registerModal(dialog: HTMLDialogElement) {
  dialogs.push(dialog);
  for (const notify of listeners) notify();
  return () => {
    const index = dialogs.indexOf(dialog);
    if (index >= 0) dialogs.splice(index, 1);
    for (const notify of listeners) notify();
  };
}
export function notificationPortal(node: HTMLElement) {
  const parent = node.parentNode;
  function move() {
    (dialogs.at(-1) ?? parent)?.appendChild(node);
  }
  listeners.add(move);
  move();
  return {
    destroy() {
      listeners.delete(move);
      node.remove();
    },
  };
}

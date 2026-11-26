import { iconSvg, isIconName } from '../design/icons';
import { TRAY_ITEMS } from './tray-copy';

/** Replace [data-icon] placeholders with the shared icon set. */
export function mountIcons(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>('[data-icon]').forEach((el) => {
    const name = el.dataset.icon || '';
    if (!isIconName(name)) return;
    el.innerHTML = iconSvg(name, 16);
  });
}

/** Fill the settings menu-bar preview from the same labels the tray uses. */
export function mountTrayPreview(root: ParentNode = document): void {
  const list = root.querySelector('#menu-preview');
  if (!list) return;
  list.replaceChildren();
  for (const item of TRAY_ITEMS) {
    const li = document.createElement('li');
    li.dataset.trayId = item.id;
    li.textContent = item.label;
    list.appendChild(li);
  }
}

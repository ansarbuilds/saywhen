/** Menu-bar and tray wording. scripts/ui-views.test.mjs checks these strings
    against src-tauri/src/tray_menu.rs so the two lists cannot drift. */

export interface TrayItem {
  id: string;
  label: string;
}

export const TRAY_ITEMS: readonly TrayItem[] = [
  { id: 'open', label: 'Open saywhen' },
  { id: 'camera', label: 'Pause or resume the camera' },
  { id: 'hud', label: 'Show or hide the pill' },
  { id: 'place', label: 'Move the pill to this display' },
  { id: 'cancel', label: 'Cancel the current line' },
  { id: 'quit', label: 'Quit saywhen' },
];

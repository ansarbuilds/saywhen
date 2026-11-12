// Command palette: a keyboard-first surface for navigation and actions.
// It only clicks existing controls, so every command has the same behavior as
// the button it represents and no backend contract is duplicated here.
import { openIntro } from './intro';

type Command = {
  id: string;
  label: string;
  detail: string;
  section: 'Go' | 'Act' | 'Look' | 'Help';
  keys?: string;
  run: () => void;
  available?: () => boolean;
};

const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const modLabel = isMac ? '⌘K' : 'Ctrl K';

function click(selector: string) {
  document.querySelector<HTMLElement>(selector)?.click();
}

function clickNav(tab: string) {
  click(`.nav-item[data-tab="${tab}"]`);
}

function chooseTheme(theme: 'light' | 'dark' | 'system') {
  click(`[data-theme-choice="${theme}"]`);
}

function chooseZoom(value: string) {
  click(`[data-ui-zoom="${value}"]`);
}

function buildCommands(): Command[] {
  return [
    {
      id: 'go-home',
      label: 'Go to Home',
      detail: 'Talk, check, send',
      section: 'Go',
      keys: isMac ? '⌘1' : 'Ctrl 1',
      run: () => clickNav('home'),
    },
    {
      id: 'go-settings',
      label: 'Open Settings',
      detail: 'Listening, sending, words, privacy',
      section: 'Go',
      keys: isMac ? '⌘2' : 'Ctrl 2',
      run: () => clickNav('preferences'),
    },
    {
      id: 'go-diagnostics',
      label: 'Open Diagnostics',
      detail: 'Live numbers and the practice box',
      section: 'Go',
      keys: isMac ? '⌘3' : 'Ctrl 3',
      run: () => clickNav('lab'),
    },
    {
      id: 'go-guide',
      label: 'Open the Guide',
      detail: 'How to use saywhen',
      section: 'Go',
      keys: isMac ? '⌘4' : 'Ctrl 4',
      run: () => clickNav('guide'),
    },
    {
      id: 'start',
      label: 'Start listening',
      detail: 'Begin a manual recording',
      section: 'Act',
      run: () => click('#start-listen-btn'),
      available: () => !(document.getElementById('start-listen-btn') as HTMLButtonElement | null)?.disabled,
    },
    {
      id: 'stop',
      label: 'Stop and turn into text',
      detail: 'Finish the current recording',
      section: 'Act',
      run: () => click('#stop-listen-btn'),
      available: () => !(document.getElementById('stop-listen-btn') as HTMLButtonElement | null)?.disabled,
    },
    {
      id: 'cancel',
      label: 'Cancel the current utterance',
      detail: 'Throw this one away',
      section: 'Act',
      run: () => click('#cancel-listen-btn'),
      available: () => !(document.getElementById('cancel-listen-btn') as HTMLButtonElement | null)?.disabled,
    },
    {
      id: 'camera',
      label: 'Toggle the camera gate',
      detail: 'Turn hands-free listening on or off',
      section: 'Act',
      run: () => click('#toggle-camera-btn'),
    },
    {
      id: 'calibrate',
      label: 'Calibrate the camera',
      detail: 'Look at your screen for 1.5 seconds',
      section: 'Act',
      run: () => click('#calibrate-vision-btn'),
    },
    {
      id: 'paste-last',
      label: 'Paste the last transcript',
      detail: 'Type it again without pressing Enter',
      section: 'Act',
      run: () => click('#paste-last-btn'),
    },
    {
      id: 'setup',
      label: 'Open the setup guide',
      detail: 'Permissions, model and camera checklist',
      section: 'Act',
      run: () => click('#onboarding-open-btn'),
    },
    {
      id: 'intro',
      label: 'Replay the intro',
      detail: 'Look, talk, sent in three seconds',
      section: 'Look',
      run: openIntro,
    },
    {
      id: 'demo',
      label: 'Play the 7-second demo',
      detail: 'Look, talk, check, sent',
      section: 'Look',
      run: () => click('[data-premiere-open]'),
    },
    {
      id: 'theme-light',
      label: 'Use the light theme',
      detail: 'Warm paper and plum',
      section: 'Look',
      run: () => chooseTheme('light'),
    },
    {
      id: 'theme-dark',
      label: 'Use the dark theme',
      detail: 'Low light, bright text',
      section: 'Look',
      run: () => chooseTheme('dark'),
    },
    {
      id: 'theme-system',
      label: 'Follow the system theme',
      detail: 'Match your Mac or PC',
      section: 'Look',
      run: () => chooseTheme('system'),
    },
    {
      id: 'motion',
      label: 'Toggle reduce motion',
      detail: 'Fewer transitions and no drifting glow',
      section: 'Look',
      run: () => click('#toggle-reduce-motion'),
    },
    {
      id: 'contrast',
      label: 'Toggle high contrast',
      detail: 'Black, white and amber',
      section: 'Look',
      run: () => click('#toggle-high-contrast'),
    },
    {
      id: 'zoom-default',
      label: 'Reset text size',
      detail: 'Back to the default window scale',
      section: 'Look',
      run: () => chooseZoom('1'),
    },
    {
      id: 'zoom-large',
      label: 'Use large text',
      detail: 'Scale the whole window up',
      section: 'Look',
      run: () => chooseZoom('1.1'),
    },
    {
      id: 'diagnostics',
      label: 'Copy diagnostics',
      detail: 'State, permissions, model and recent problems',
      section: 'Help',
      run: () => click('#copy-diagnostics-btn'),
    },
    {
      id: 'faq',
      label: 'Copy the FAQ link',
      detail: 'Share or open the answers',
      section: 'Help',
      run: () => click('#copy-docs-link-btn'),
    },
    {
      id: 'clear-history',
      label: 'Clear recent history',
      detail: 'Delete the local transcript list',
      section: 'Help',
      run: () => click('#clear-feed-btn'),
    },
  ];
}

export function initCommandPalette() {
  const root = document.getElementById('command-palette') as HTMLElement;
  const input = document.getElementById('command-input') as HTMLInputElement;
  const list = document.getElementById('command-list') as HTMLElement;
  const openButton = document.getElementById('command-open-btn');

  const commands = buildCommands();
  let filtered: Command[] = [];
  let active = 0;
  let returnFocus: HTMLElement | null = null;

  const keyLabel = document.getElementById('command-key-label');
  if (keyLabel) keyLabel.textContent = modLabel;

  function render() {
    const query = input.value.trim().toLowerCase();
    filtered = commands.filter((command) => {
      if (command.available && !command.available()) return false;
      if (!query) return true;
      return `${command.label} ${command.detail} ${command.section}`.toLowerCase().includes(query);
    });
    active = Math.min(active, Math.max(0, filtered.length - 1));
    list.innerHTML = '';
    if (filtered.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'command-empty';
      empty.textContent = 'No command matches that.';
      list.appendChild(empty);
      input.removeAttribute('aria-activedescendant');
      return;
    }
    let lastSection = '';
    filtered.forEach((command, index) => {
      if (command.section !== lastSection) {
        lastSection = command.section;
        const heading = document.createElement('div');
        heading.className = 'command-section';
        heading.setAttribute('role', 'presentation');
        heading.textContent = command.section;
        list.appendChild(heading);
      }
      const item = document.createElement('button');
      item.type = 'button';
      item.className = `command-item${index === active ? ' is-active' : ''}`;
      item.id = `command-item-${index}`;
      item.setAttribute('role', 'option');
      item.setAttribute('aria-selected', index === active ? 'true' : 'false');
      item.innerHTML = `<span class="command-label">${command.label}</span><span class="command-detail">${command.detail}</span>${command.keys ? `<kbd>${command.keys}</kbd>` : ''}`;
      item.addEventListener('mouseenter', () => {
        active = index;
        updateActive();
      });
      item.addEventListener('click', () => execute(index));
      list.appendChild(item);
    });
    updateActive();
  }

  function updateActive() {
    const items = Array.from(list.querySelectorAll<HTMLElement>('.command-item'));
    items.forEach((item, index) => {
      const on = index === active;
      item.classList.toggle('is-active', on);
      item.setAttribute('aria-selected', on ? 'true' : 'false');
      if (on) {
        input.setAttribute('aria-activedescendant', item.id);
        item.scrollIntoView({ block: 'nearest' });
      }
    });
  }

  function open() {
    if (root.classList.contains('hidden') === false) return;
    if (document.querySelector('.modal-backdrop:not(.hidden), .premiere:not(.hidden), .intro:not(.hidden)')) return;
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    root.classList.remove('hidden');
    input.value = '';
    active = 0;
    render();
    requestAnimationFrame(() => input.focus());
  }

  function close() {
    root.classList.add('hidden');
    if (returnFocus && document.contains(returnFocus)) returnFocus.focus();
    returnFocus = null;
  }

  function execute(index: number) {
    const command = filtered[index];
    if (!command) return;
    close();
    command.run();
  }

  input.addEventListener('input', () => {
    active = 0;
    render();
  });
  input.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (filtered.length === 0) return;
      active = (active + (event.key === 'ArrowDown' ? 1 : -1) + filtered.length) % filtered.length;
      updateActive();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      execute(active);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'Tab') {
      event.preventDefault();
      input.focus();
    }
  });
  root.addEventListener('click', (event) => {
    if (event.target === root) close();
  });
  openButton?.addEventListener('click', open);
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && !root.classList.contains('hidden')) {
      event.preventDefault();
      close();
      return;
    }
    const mod = isMac ? event.metaKey : event.ctrlKey;
    if (mod && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      if (root.classList.contains('hidden')) open();
      else close();
    }
  });
}

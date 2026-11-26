import { invoke } from '@tauri-apps/api/core';

interface LlmApiKeyStatus {
  present: boolean;
  backend: string;
}

function describeKeyStatus(status: LlmApiKeyStatus): string {
  if (!status.present || status.backend === 'none') {
    return 'No key saved. A local Ollama address does not need one.';
  }
  if (status.backend === 'keyring') {
    return 'A key is saved in the system keychain. It is not written into settings.';
  }
  return 'A key is saved in a private file on this computer. It is not written into settings.';
}

export function initByokSettings(): void {
  const input = document.getElementById('llm-api-key-input') as HTMLInputElement | null;
  const save = document.getElementById('llm-api-key-save') as HTMLButtonElement | null;
  const clear = document.getElementById('llm-api-key-clear') as HTMLButtonElement | null;
  const status = document.getElementById('llm-key-status');
  if (!input || !save || !clear || !status) return;

  const refresh = async () => {
    try {
      const snap = await invoke<LlmApiKeyStatus>('llm_api_key_status');
      status.textContent = describeKeyStatus(snap);
    } catch {
      status.textContent = 'Key status is available in the desktop app.';
    }
  };

  save.addEventListener('click', async () => {
    const apiKey = input.value.trim();
    if (!apiKey) {
      status.textContent = 'Paste a key first, or remove the saved one.';
      return;
    }
    try {
      await invoke('set_llm_api_key', { apiKey });
      input.value = '';
      await refresh();
    } catch (err) {
      status.textContent = String(err);
    }
  });

  clear.addEventListener('click', async () => {
    try {
      await invoke('clear_llm_api_key');
      input.value = '';
      await refresh();
    } catch (err) {
      status.textContent = String(err);
    }
  });

  void refresh();
}

import { invoke } from '@tauri-apps/api/core';

interface LicenseStatus {
  tier: string;
  enforcement: string;
  trialDaysLeft: number;
  checker: string;
  message: string;
  keyPresent: boolean;
}

function describe(status: LicenseStatus): string {
  const tier = status.tier === 'pro' ? 'Pro' : status.tier === 'trial' ? 'Trial' : 'Free';
  return `${tier}. ${status.message}`;
}

export function initLicenseSettings(): void {
  const input = document.getElementById('license-key-input') as HTMLInputElement | null;
  const save = document.getElementById('license-key-save');
  const clear = document.getElementById('license-key-clear');
  const status = document.getElementById('license-status');
  if (!input || !save || !clear || !status) return;

  const refresh = async () => {
    try {
      const snap = await invoke<LicenseStatus>('license_status');
      status.textContent = describe(snap);
    } catch {
      status.textContent = 'License status is available in the desktop app.';
    }
  };

  save.addEventListener('click', async () => {
    const licenseKey = input.value.trim();
    if (!licenseKey) {
      status.textContent = 'Paste a license key first.';
      return;
    }
    try {
      const snap = await invoke<LicenseStatus>('set_license_key', { licenseKey });
      input.value = '';
      status.textContent = describe(snap);
    } catch (err) {
      status.textContent = String(err);
    }
  });

  clear.addEventListener('click', async () => {
    try {
      const snap = await invoke<LicenseStatus>('clear_license_key');
      input.value = '';
      status.textContent = describe(snap);
    } catch (err) {
      status.textContent = String(err);
    }
  });

  void refresh();
}

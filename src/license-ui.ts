import { invoke } from '@tauri-apps/api/core';
import { licenseView } from './ui/product-views';

interface LicenseStatus {
  tier: string;
  enforcement: string;
  trialDaysLeft: number;
  checker: string;
  message: string;
  keyPresent: boolean;
}

function paintLicense(view: ReturnType<typeof licenseView>, detailOverride?: string): void {
  const status = document.getElementById('license-status');
  const eyebrow = document.getElementById('license-eyebrow');
  const headline = document.getElementById('license-headline');
  const detail = document.getElementById('license-detail');
  if (eyebrow) eyebrow.textContent = view.eyebrow;
  if (headline) headline.textContent = view.headline;
  if (detail) detail.textContent = detailOverride ?? view.detail;
  if (status) status.dataset.tier = view.eyebrow.toLowerCase();
}

function describe(status: LicenseStatus): string {
  const view = licenseView({
    tier: status.tier,
    trialDaysLeft: status.trialDaysLeft,
    checker: status.checker,
    message: status.message,
  });
  paintLicense(view);
  return `${view.eyebrow}. ${view.headline} ${view.detail}`;
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
      describe(snap);
    } catch {
      paintLicense(
        licenseView({ tier: 'trial', trialDaysLeft: 14, checker: 'development', message: '' }),
        'This preview shows the 14-day trial copy. The desktop app checks the real trial. Enforcement is off, so nothing is locked.',
      );
    }
  };

  save.addEventListener('click', async () => {
    const licenseKey = input.value.trim();
    if (!licenseKey) {
      paintLicense({ eyebrow: 'License', headline: 'Paste a license key first.', detail: '', action: 'Add a license key' });
      return;
    }
    try {
      const snap = await invoke<LicenseStatus>('set_license_key', { licenseKey });
      input.value = '';
      describe(snap);
    } catch (err) {
      paintLicense({ eyebrow: 'License', headline: 'The key was not saved.', detail: String(err), action: 'Add a license key' });
    }
  });

  clear.addEventListener('click', async () => {
    try {
      const snap = await invoke<LicenseStatus>('clear_license_key');
      input.value = '';
      describe(snap);
    } catch (err) {
      paintLicense({ eyebrow: 'License', headline: 'The key is still saved.', detail: String(err), action: 'Add a license key' });
    }
  });

  void refresh();
}

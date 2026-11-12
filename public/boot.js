try {
  var themeChoice = localStorage.getItem('saywhen-theme') || 'system';
  var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  var dark = themeChoice === 'dark' || (themeChoice === 'system' && prefersDark);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  document.documentElement.dataset.themeChoice = themeChoice;
  document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
  var contrast = localStorage.getItem('saywhen-contrast') === 'high' ? 'high' : 'normal';
  document.documentElement.dataset.contrast = contrast;
  var motion = localStorage.getItem('saywhen-motion') === 'reduced' ? 'reduced' : 'full';
  document.documentElement.dataset.motion = motion;
} catch (error) {}

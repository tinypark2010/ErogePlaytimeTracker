const repository = 'tinypark2010/ErogePlaytimeTracker';
const releasesUrl = `https://github.com/${repository}/releases/latest`;

export function findInstaller(release) {
  return release.assets?.find((asset) =>
    /^ErogePlaytimeTracker_.+_x64-setup\.exe$/i.test(asset.name),
  );
}

async function loadLatestRelease() {
  const releaseLabels = document.querySelectorAll('[data-release]');

  try {
    const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
      headers: { Accept: 'application/vnd.github+json' },
    });

    if (!response.ok) throw new Error(`GitHub API returned ${response.status}`);

    const release = await response.json();
    const installer = findInstaller(release);

    document.querySelectorAll('[data-download]').forEach((link) => {
      link.href = installer?.browser_download_url ?? release.html_url ?? releasesUrl;
      link.setAttribute(
        'aria-label',
        installer
          ? `Eroge Playtime Tracker ${release.tag_name}をダウンロード`
          : `Eroge Playtime Tracker ${release.tag_name}のリリースページを開く`,
      );
    });
    releaseLabels.forEach((label) => {
      label.textContent = `最新版 ${release.tag_name}`;
    });
  } catch {
    releaseLabels.forEach((label) => {
      label.textContent = '最新版はReleasesで確認できます';
    });
  }
}

if (typeof document !== 'undefined') {
  document.querySelectorAll('[data-year]').forEach((year) => {
    year.textContent = String(new Date().getFullYear());
  });

  loadLatestRelease();
}

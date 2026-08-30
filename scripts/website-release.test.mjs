import { describe, expect, it } from 'vitest';
import { findInstaller } from '../website/release.js';

describe('website release download', () => {
  it('selects the Windows x64 installer and ignores its signature', () => {
    const installer = findInstaller({
      assets: [
        {
          name: 'ErogePlaytimeTracker_0.1.9_x64-setup.exe.sig',
          browser_download_url: 'signature',
        },
        {
          name: 'ErogePlaytimeTracker_0.1.9_x64-setup.exe',
          browser_download_url: 'installer',
        },
      ],
    });

    expect(installer?.browser_download_url).toBe('installer');
  });

  it('returns undefined when a release has no matching installer', () => {
    expect(findInstaller({ assets: [{ name: 'latest.json' }] })).toBeUndefined();
    expect(findInstaller({})).toBeUndefined();
  });
});

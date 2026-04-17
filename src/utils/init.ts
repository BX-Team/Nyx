import { getVersion } from './ipc';

export type Platform = 'win32' | 'linux' | 'darwin';

function detectPlatform(): Platform {
  const ua = navigator.userAgent;
  if (ua.includes('Windows')) return 'win32';
  if (ua.includes('Linux')) return 'linux';
  return 'darwin';
}

export const platform: Platform = detectPlatform();
export let version: string = '';

export async function init(): Promise<void> {
  version = await getVersion();
}

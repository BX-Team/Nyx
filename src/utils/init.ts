import { getVersion } from './ipc';

function detectPlatform(): NodeJS.Platform {
  const ua = navigator.userAgent;
  if (ua.includes('Windows')) return 'win32';
  if (ua.includes('Linux')) return 'linux';
  return 'darwin';
}

export const platform: NodeJS.Platform = detectPlatform();
export let version: string = '';

export async function init(): Promise<void> {
  version = await getVersion();
}

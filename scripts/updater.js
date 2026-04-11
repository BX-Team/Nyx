import { writeFileSync } from 'node:fs';

const API_BASE = 'https://api.github.com';

function requiredEnv(name) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

function parseRepository() {
  const repository = requiredEnv('GITHUB_REPOSITORY');
  const [owner, repo] = repository.split('/');
  if (!owner || !repo) {
    throw new Error(`Invalid GITHUB_REPOSITORY value: ${repository}`);
  }
  return { owner, repo };
}

async function ghRequest(path, options = {}) {
  const token = requiredEnv('GITHUB_TOKEN');
  const response = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...(options.headers ?? {}),
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API request failed (${response.status}) for ${path}: ${body}`);
  }

  return response;
}

function findPlatformAssets(assets) {
  const toLower = value => value.toLowerCase();
  const find = predicate => assets.find(asset => predicate(toLower(asset.name)));

  return {
    'windows-x86_64': find(
      name =>
        name.endsWith('-setup.exe') &&
        (name.includes('_x64') || name.includes('x86_64') || name.includes('x64-setup.exe')),
    ),
    'windows-aarch64': find(
      name =>
        name.endsWith('-setup.exe') &&
        (name.includes('_arm64') || name.includes('aarch64') || name.includes('arm64-setup.exe')),
    ),
    'linux-x86_64': find(name => name.endsWith('.appimage') && (name.includes('_amd64') || name.includes('x86_64'))),
  };
}

async function getSignatureText(owner, repo, assets, assetName) {
  const signatureAsset = assets.find(asset => asset.name === `${assetName}.sig`);
  if (!signatureAsset) {
    throw new Error(`Signature file not found for asset: ${assetName}`);
  }

  const response = await ghRequest(`/repos/${owner}/${repo}/releases/assets/${signatureAsset.id}`, {
    headers: {
      Accept: 'application/octet-stream',
    },
  });

  return (await response.text()).trim();
}

async function main() {
  const { owner, repo } = parseRepository();
  const releaseTag = requiredEnv('RELEASE_TAG');
  const version = requiredEnv('VERSION');
  const notes = process.env.RELEASE_NOTES || `Release ${releaseTag}`;

  const releaseResponse = await ghRequest(`/repos/${owner}/${repo}/releases/tags/${encodeURIComponent(releaseTag)}`);
  const release = await releaseResponse.json();
  const assets = Array.isArray(release.assets) ? release.assets : [];

  const platformAssets = findPlatformAssets(assets);
  const missing = Object.entries(platformAssets)
    .filter(([, asset]) => !asset)
    .map(([platform]) => platform);

  if (missing.length > 0) {
    const available = assets.map(asset => asset.name).join(', ');
    throw new Error(`Missing required updater assets for: ${missing.join(', ')}. Available assets: ${available}`);
  }

  const platforms = {};
  for (const [platform, asset] of Object.entries(platformAssets)) {
    platforms[platform] = {
      url: asset.browser_download_url,
      signature: await getSignatureText(owner, repo, assets, asset.name),
    };
  }

  const payload = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms,
  };

  writeFileSync('update.json', `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(`Generated update.json for ${releaseTag}`);
}

main().catch(error => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});

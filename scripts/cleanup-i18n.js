#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync, statSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join, basename, extname, dirname } from 'node:path';
import { transpileModule, ModuleKind, ScriptTarget } from 'typescript';

const ROOT_DIR = resolve(__dirname, '..');
const LOCALES_DIR = resolve(ROOT_DIR, 'src/locales');
const DEFAULT_SOURCE_DIRS = [resolve(ROOT_DIR, 'src'), resolve(ROOT_DIR, 'src-tauri')];
const DEFAULT_BASELINE = 'en-US';

const SUPPORTED_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.rs']);
const IGNORE_DIR_NAMES = new Set([
  '.git',
  '.idea',
  '.vscode',
  'node_modules',
  'dist',
  'build',
  'coverage',
  'target',
  '.next',
  '.cache',
]);

const KEY_PATTERN = /^[A-Za-z][A-Za-z0-9_-]*(?:\.[A-Za-z0-9_-]+)+$/;
const WHITELIST_KEYS = new Set(['_version']);
const MAX_PREVIEW = 40;

function printUsage() {
  console.log(`Usage: bun scripts/cleanup-i18n.js [options]

Options:
  --apply            Write locale files with unused keys removed
  --baseline <lang>  Baseline locale folder name (default: ${DEFAULT_BASELINE})
  --src <path>       Include an extra source directory (repeatable)
  --report <path>    Write JSON report to a file
  --help             Show this help
`);
}

function parseArgs(argv) {
  const options = {
    apply: false,
    baseline: DEFAULT_BASELINE,
    extraSources: [],
    reportPath: null,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    switch (arg) {
      case '--apply':
        options.apply = true;
        break;
      case '--baseline': {
        const value = argv[i + 1];
        if (!value) throw new Error('--baseline requires a locale name');
        options.baseline = value;
        i += 1;
        break;
      }
      case '--src': {
        const value = argv[i + 1];
        if (!value) throw new Error('--src requires a directory path');
        options.extraSources.push(resolve(process.cwd(), value));
        i += 1;
        break;
      }
      case '--report': {
        const value = argv[i + 1];
        if (!value) throw new Error('--report requires a file path');
        options.reportPath = resolve(process.cwd(), value);
        i += 1;
        break;
      }
      case '--help':
        printUsage();
        process.exit(0);
        break;
      default:
        throw new Error(`Unknown option: ${arg}`);
    }
  }

  return options;
}

function listLocaleFiles() {
  if (!existsSync(LOCALES_DIR)) {
    throw new Error(`Locales directory not found: ${LOCALES_DIR}`);
  }

  const entries = readdirSync(LOCALES_DIR, { withFileTypes: true });
  const locales = [];

  for (const entry of entries) {
    if (!entry.isDirectory() || entry.name.startsWith('.')) {
      continue;
    }

    const localeFile = join(LOCALES_DIR, entry.name, 'index.ts');
    if (!existsSync(localeFile)) {
      console.warn(`Skip locale without index.ts: ${entry.name}`);
      continue;
    }

    locales.push({
      name: entry.name,
      filePath: localeFile,
      data: loadTsDefaultExport(localeFile),
    });
  }

  locales.sort((a, b) => a.name.localeCompare(b.name));
  return locales;
}

function loadTsDefaultExport(filePath) {
  const source = readFileSync(filePath, 'utf8');
  const transpiled = transpileModule(source, {
    compilerOptions: {
      module: ModuleKind.CommonJS,
      target: ScriptTarget.ES2022,
    },
    fileName: filePath,
    reportDiagnostics: false,
  }).outputText;

  const module = { exports: {} };
  const fn = new Function('module', 'exports', transpiled);
  fn(module, module.exports);

  const value = _default ?? module.exports;
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`Locale in ${filePath} does not export an object`);
  }

  return value;
}

function collectSourceFiles(sourceDirs) {
  const result = [];
  const seen = new Set();

  for (const sourceDir of sourceDirs) {
    if (!existsSync(sourceDir)) continue;

    const stack = [sourceDir];
    while (stack.length > 0) {
      const current = stack.pop();
      if (!current) continue;

      if (seen.has(current)) continue;
      seen.add(current);

      let stat;
      try {
        stat = statSync(current);
      } catch {
        continue;
      }

      if (stat.isDirectory()) {
        const baseName = basename(current);
        if (IGNORE_DIR_NAMES.has(baseName)) continue;
        if (current.startsWith(LOCALES_DIR)) continue;

        const entries = readdirSync(current, { withFileTypes: true });
        for (const entry of entries) {
          if (entry.isSymbolicLink()) continue;
          stack.push(join(current, entry.name));
        }
      } else if (stat.isFile()) {
        if (current.startsWith(LOCALES_DIR)) continue;
        const ext = extname(current).toLowerCase();
        if (!SUPPORTED_EXTENSIONS.has(ext)) continue;

        result.push({
          path: current,
          content: readFileSync(current, 'utf8'),
        });
      }
    }
  }

  result.sort((a, b) => a.path.localeCompare(b.path));
  return result;
}

function collectUsedKeys(sourceFiles) {
  const usedKeys = new Set();
  const keyRegex = /['"`]([A-Za-z][A-Za-z0-9_-]*(?:\.[A-Za-z0-9_-]+)+)['"`]/g;

  for (const file of sourceFiles) {
    let match = keyRegex.exec(file.content);
    while (match !== null) {
      const key = match[1];
      if (KEY_PATTERN.test(key)) {
        usedKeys.add(key);
      }
      match = keyRegex.exec(file.content);
    }
  }

  return usedKeys;
}

function flattenLocale(input, prefix = '', map = new Map()) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return map;
  }

  for (const [key, value] of Object.entries(input)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      flattenLocale(value, fullKey, map);
    } else {
      map.set(fullKey, value);
    }
  }

  return map;
}

function removeKey(target, dottedKey) {
  const parts = dottedKey.split('.').filter(Boolean);
  if (parts.length === 0) return;

  let current = target;
  for (let i = 0; i < parts.length; i += 1) {
    const part = parts[i];
    if (!current || typeof current !== 'object' || !(part in current)) {
      return;
    }

    if (i === parts.length - 1) {
      delete current[part];
      return;
    }

    current = current[part];
  }
}

function cleanupEmptyBranches(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return false;
  }

  for (const key of Object.keys(input)) {
    if (cleanupEmptyBranches(input[key])) {
      delete input[key];
    }
  }

  return Object.keys(input).length === 0;
}

function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

function escapeString(value) {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/'/g, "\\'")
    .replace(/\r/g, '\\r')
    .replace(/\n/g, '\\n')
    .replace(/\t/g, '\\t');
}

function isIdentifier(value) {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(value);
}

function toTsLiteral(value, indent = 0) {
  const pad = ' '.repeat(indent);
  const nextPad = ' '.repeat(indent + 2);

  if (typeof value === 'string') {
    return `'${escapeString(value)}'`;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  if (value === null) {
    return 'null';
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return '[]';
    const lines = value.map(item => `${nextPad}${toTsLiteral(item, indent + 2)},`);
    return `[\n${lines.join('\n')}\n${pad}]`;
  }
  if (!value || typeof value !== 'object') {
    return 'undefined';
  }

  const entries = Object.entries(value);
  if (entries.length === 0) {
    return '{}';
  }

  const lines = entries.map(([key, child]) => {
    const serializedKey = isIdentifier(key) ? key : `'${escapeString(key)}'`;
    return `${nextPad}${serializedKey}: ${toTsLiteral(child, indent + 2)},`;
  });

  return `{\n${lines.join('\n')}\n${pad}}`;
}

function writeLocale(filePath, localeData) {
  const content = `export default ${toTsLiteral(localeData, 0)}\n`;
  writeFileSync(filePath, content, 'utf8');
}

function preview(label, keys) {
  if (keys.length === 0) return;
  for (const key of keys.slice(0, MAX_PREVIEW)) {
    console.log(`  - ${label}: ${key}`);
  }
  if (keys.length > MAX_PREVIEW) {
    console.log(`  - ${label}: ... and ${keys.length - MAX_PREVIEW} more`);
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const sourceDirs = [...new Set([...DEFAULT_SOURCE_DIRS, ...options.extraSources])];

  const locales = listLocaleFiles();
  if (locales.length === 0) {
    console.log('No locale files found.');
    return;
  }

  const baseline = locales.find(locale => locale.name.toLowerCase() === options.baseline.toLowerCase());
  if (!baseline) {
    const available = locales.map(locale => locale.name).join(', ');
    throw new Error(`Baseline locale "${options.baseline}" not found. Available: ${available}`);
  }

  console.log('Scanning source directories:');
  for (const dir of sourceDirs) {
    console.log(`- ${dir}`);
  }

  const sourceFiles = collectSourceFiles(sourceDirs);
  const usedKeys = collectUsedKeys(sourceFiles);
  const baselineFlat = flattenLocale(baseline.data);
  const baselineKeys = new Set(baselineFlat.keys());

  const results = [];

  const sortedLocales = locales.slice().sort((a, b) => {
    if (a.name === baseline.name) return -1;
    if (b.name === baseline.name) return 1;
    return a.name.localeCompare(b.name);
  });

  for (const locale of sortedLocales) {
    const flattened = flattenLocale(locale.data);
    const unusedKeys = [];

    for (const key of flattened.keys()) {
      if (WHITELIST_KEYS.has(key)) continue;
      if (!usedKeys.has(key)) {
        unusedKeys.push(key);
      }
    }

    const missingKeys = [];
    for (const key of baselineKeys) {
      if (!flattened.has(key)) {
        missingKeys.push(key);
      }
    }

    unusedKeys.sort();
    missingKeys.sort();

    console.log(
      `\n[${locale.name}] unused=${unusedKeys.length}, missing=${missingKeys.length}, total=${flattened.size}`,
    );
    preview('unused', unusedKeys);

    if (options.apply && unusedKeys.length > 0) {
      const updated = deepClone(locale.data);
      for (const key of unusedKeys) {
        removeKey(updated, key);
      }
      cleanupEmptyBranches(updated);
      writeLocale(locale.filePath, updated);
      locale.data = updated;
      console.log(`[${locale.name}] cleaned (${unusedKeys.length} keys removed)`);
    }

    results.push({
      locale: locale.name,
      file: locale.filePath,
      totalKeys: flattened.size,
      unusedKeys,
      missingKeys,
      removed: options.apply ? unusedKeys : [],
    });
  }

  const totals = results.reduce(
    (acc, item) => {
      acc.unused += item.unusedKeys.length;
      acc.missing += item.missingKeys.length;
      return acc;
    },
    { unused: 0, missing: 0 },
  );

  console.log(`\nTotals -> unused: ${totals.unused}, missing: ${totals.missing}`);

  if (options.apply) {
    console.log('Locale files updated. Review changes before commit.');
  } else {
    console.log('Dry-run mode. Add --apply to remove keys.');
  }

  if (options.reportPath) {
    const report = {
      generatedAt: new Date().toISOString(),
      options: {
        apply: options.apply,
        baseline: options.baseline,
      },
      sourceDirs,
      totals,
      results,
    };

    mkdirSync(dirname(options.reportPath), { recursive: true });
    writeFileSync(options.reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log(`Report written to ${options.reportPath}`);
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}

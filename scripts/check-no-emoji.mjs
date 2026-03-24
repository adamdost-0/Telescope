#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

const ROOT = process.cwd();
const DEFAULT_DIRS = [
  'apps/web/src',
  'apps/web/tests-e2e',
  'docs',
  '.github',
  '.squad',
  'AGENTS.md',
  'apps/web/AGENTS.md',
  '.github/agents/squad.agent.md',
  '.squad/templates/squad.agent.md',
];

// Emoji detection: Extended_Pictographic covers nearly all emoji glyphs.
// Also catch legacy symbol ranges often used as emoji (⚠️, ✅, etc.).
const EMOJI_REGEX = /\p{Extended_Pictographic}/u;

// Optional allowlist (substring match). Keep tight.
const ALLOWLIST = [
  // Add entries if certain files must retain emoji (currently empty)
];

const IGNORED_DIRS = new Set([
  'node_modules',
  '.svelte-kit',
  'build',
  'dist',
  'target',
  '.git',
  '.idea',
  '.vscode',
  'apps/desktop/dist',
  'apps/web/build',
]);

const TEXT_EXTENSIONS = new Set([
  '.ts', '.tsx', '.js', '.jsx', '.svelte', '.md', '.mdx', '.json', '.yml', '.yaml', '.toml', '.rs', '.txt', '.sh', '.mjs', '.cjs', '.tsv', '.csv', '.html', '.css', '.scss'
]);

function isAllowed(filePath) {
  return ALLOWLIST.some((entry) => filePath.includes(entry));
}

function shouldScan(filePath) {
  const stat = fs.statSync(filePath);
  if (stat.isDirectory()) return !IGNORED_DIRS.has(path.relative(ROOT, filePath));
  const ext = path.extname(filePath);
  if (!TEXT_EXTENSIONS.has(ext) && ext !== '') return false;
  return true;
}

function walk(filePath, matches) {
  const stat = fs.statSync(filePath);
  if (stat.isDirectory()) {
    const entries = fs.readdirSync(filePath);
    for (const entry of entries) {
      const child = path.join(filePath, entry);
      if (!shouldScan(child)) continue;
      walk(child, matches);
    }
    return;
  }

  if (!shouldScan(filePath)) return;
  if (isAllowed(filePath)) return;

  const content = fs.readFileSync(filePath, 'utf8');
  if (!EMOJI_REGEX.test(content)) return;

  const lines = content.split(/\r?\n/);
  lines.forEach((line, idx) => {
    if (EMOJI_REGEX.test(line)) {
      matches.push({ file: path.relative(ROOT, filePath), line: idx + 1, text: line.trim() });
    }
  });
}

async function main() {
  if (process.env.ALLOW_EMOJI === '1') {
    console.log('[check-no-emoji] Skipped (ALLOW_EMOJI=1).');
    return;
  }

  const targets = DEFAULT_DIRS.map((p) => path.resolve(ROOT, p)).filter(fs.existsSync);
  const matches = [];
  for (const target of targets) {
    walk(target, matches);
  }

  if (matches.length > 0) {
    console.error('\n[FAIL] Emoji detected in source/docs:');
    matches.forEach((m) => {
      console.error(` - ${m.file}:${m.line}: ${m.text}`);
    });
    console.error('\nFix: replace emojis with standard icon components, checkboxes (- [x]), or plain text.');
    process.exitCode = 1;
  } else {
    console.log('[check-no-emoji] OK: no emoji found');
  }
}

await main();

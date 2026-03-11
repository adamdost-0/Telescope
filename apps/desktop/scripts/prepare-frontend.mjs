// Prepare the SvelteKit web app as the desktop frontend.
// Builds apps/web and copies static output to dist/ for Tauri.

import { execSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const desktopRoot = resolve(__dirname, '..');
const webRoot = resolve(desktopRoot, '..', 'web');
const webBuild = resolve(webRoot, 'build');
const dist = resolve(desktopRoot, 'dist');

console.log('[prepare-frontend] Building web app...');
execSync('pnpm run build', { cwd: webRoot, stdio: 'inherit' });

console.log('[prepare-frontend] Copying build output to dist/...');
if (existsSync(dist)) {
  rmSync(dist, { recursive: true });
}
mkdirSync(dist, { recursive: true });
cpSync(webBuild, dist, { recursive: true });

console.log('[prepare-frontend] Done.');

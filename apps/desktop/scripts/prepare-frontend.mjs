import { mkdir, copyFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const appDir = join(__dirname, '..');
const src = join(appDir, 'index.html');
const distDir = join(appDir, 'dist');
const dest = join(distDir, 'index.html');

await mkdir(distDir, { recursive: true });
await copyFile(src, dest);
console.log(`Prepared desktop frontend: ${dest}`);

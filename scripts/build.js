/**
 * Build script for Tojo Assistant Electron app.
 * Compiles TypeScript → JavaScript using tsc (main/preload) and esbuild (renderer).
 */

const { execSync } = require('child_process');
const esbuild = require('esbuild');
const path = require('path');
const fs = require('fs');

const ROOT = path.join(__dirname, '..');
const COMPILED_DIR = path.join(ROOT, 'electron', 'compiled');

// Clean compiled output
if (fs.existsSync(COMPILED_DIR)) {
  fs.rmSync(COMPILED_DIR, { recursive: true });
}
fs.mkdirSync(COMPILED_DIR, { recursive: true });

// 1. Compile main process (main.ts + preload.ts) with tsc
console.log('[build] Compiling main process with tsc...');
execSync('npx tsc -p tsconfig.json', { cwd: ROOT, stdio: 'inherit' });

// 2. Type-check renderer (uses DOM lib)
console.log('[build] Type-checking renderer...');
execSync('npx tsc -p tsconfig.renderer.json --noEmit', { cwd: ROOT, stdio: 'inherit' });

// 3. Bundle renderer with esbuild (all renderer modules → single IIFE bundle)
console.log('[build] Bundling renderer with esbuild...');
esbuild.buildSync({
  entryPoints: [path.join(ROOT, 'electron', 'src', 'renderer', 'app.ts')],
  bundle: true,
  outfile: path.join(ROOT, 'electron', 'renderer', 'app.js'),
  format: 'iife',
  platform: 'browser',
  target: 'es2022',
  sourcemap: true,
  minify: process.argv.includes('--production'),
});

console.log('[build] Done.');

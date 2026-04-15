#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const viteBin = resolve(appRoot, 'node_modules', 'vite', 'bin', 'vite.js');
const host = process.env.STELLAR_FORGE_UI_SMOKE_HOST || '127.0.0.1';
const port = Number(process.env.STELLAR_FORGE_UI_SMOKE_PORT || '4173');
const timeoutMs = Number(process.env.STELLAR_FORGE_UI_SMOKE_TIMEOUT_MS || '30000');
const baseUrl = `http://${host}:${port}`;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function log(message) {
  console.log(`[ui-smoke] ${message}`);
}

function wait(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

function spawnVite(args, label) {
  const child = spawn(process.execPath, [viteBin, ...args], {
    cwd: appRoot,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', (chunk) => {
    stdout += chunk.toString();
  });
  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString();
  });
  return {
    child,
    output() {
      return { label, stdout, stderr };
    },
  };
}

async function runBuild() {
  const build = spawnVite(['build'], 'build');
  const exitCode = await new Promise((resolve, reject) => {
    build.child.once('error', reject);
    build.child.once('exit', (code) => resolve(code ?? 1));
  });
  if (exitCode !== 0) {
    const output = build.output();
    throw new Error(
      `vite build failed (${exitCode})\n${output.stderr || output.stdout || 'no output'}`,
    );
  }
}

async function waitForServer(preview) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (preview.child.exitCode !== null) {
      const output = preview.output();
      throw new Error(
        `vite preview exited early\n${output.stderr || output.stdout || 'no output'}`,
      );
    }
    try {
      const response = await fetch(baseUrl);
      if (response.ok) {
        return await response.text();
      }
    } catch (_error) {
      // The preview server may still be booting.
    }
    await wait(250);
  }
  throw new Error(`timed out waiting for ${baseUrl}`);
}

function extractModulePath(html) {
  const match = html.match(/<script[^>]+src="([^"]+\.js)"/i);
  return match ? match[1] : null;
}

async function stopProcess(child) {
  if (child.exitCode !== null) {
    return;
  }
  child.kill('SIGTERM');
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    wait(1000),
  ]);
  if (child.exitCode === null) {
    child.kill('SIGKILL');
    await new Promise((resolve) => child.once('exit', resolve));
  }
}

async function main() {
  assert(
    existsSync(viteBin),
    'missing local Vite binary; run your package manager install command in apps/web first',
  );
  const generatedStatePath = resolve(appRoot, 'src', 'generated', 'stellar.ts');
  assert(
    existsSync(generatedStatePath),
    'missing generated frontend state file at src/generated/stellar.ts',
  );

  const generatedState = readFileSync(generatedStatePath, 'utf8');
  const projectName = generatedState.match(/"name":\s*"([^"]+)"/)?.[1] ?? null;
  const expectedMarkers = ['Queue', 'Runtime', 'Events', 'Contracts', 'Tokens'];

  log('building frontend bundle');
  await runBuild();

  log(`starting preview on ${baseUrl}`);
  const preview = spawnVite(
    ['preview', '--host', host, '--port', String(port), '--strictPort'],
    'preview',
  );

  try {
    const html = await waitForServer(preview);
    assert(
      html.includes('<div id="root"></div>') || html.includes('<div id="root">'),
      'preview HTML is missing the root mount',
    );
    const modulePath = extractModulePath(html);
    assert(modulePath, 'could not find the built JavaScript asset in index.html');

    const response = await fetch(new URL(modulePath, `${baseUrl}/`));
    assert(response.ok, `failed to fetch ${modulePath}: ${response.status}`);
    const bundle = await response.text();

    for (const marker of expectedMarkers) {
      assert(
        bundle.includes(marker),
        `built UI bundle is missing the marker ${JSON.stringify(marker)}`,
      );
    }
    assert(
      bundle.includes('stellar forge release '),
      'built UI bundle is missing the release action queue',
    );
    if (projectName) {
      assert(
        bundle.includes(projectName),
        `built UI bundle is missing the project name ${JSON.stringify(projectName)}`,
      );
    }

    log('UI smoke passed');
  } finally {
    await stopProcess(preview.child);
  }
}

main().catch((error) => {
  console.error(`[ui-smoke] ${error.message}`);
  process.exit(1);
});

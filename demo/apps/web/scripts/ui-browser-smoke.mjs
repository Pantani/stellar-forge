#!/usr/bin/env node
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import os from 'node:os';
import { spawn, spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const viteBin = resolve(appRoot, 'node_modules', 'vite', 'bin', 'vite.js');
const playwrightVersion =
  process.env.STELLAR_FORGE_PLAYWRIGHT_VERSION || '@playwright/test@1.59.1';
const chromiumRevision =
  process.env.STELLAR_FORGE_PLAYWRIGHT_CHROMIUM_REVISION || '1217';
const keepTemp = process.env.STELLAR_FORGE_BROWSER_SMOKE_KEEP === '1';
const host = process.env.STELLAR_FORGE_BROWSER_SMOKE_HOST || '127.0.0.1';
const port = Number(process.env.STELLAR_FORGE_BROWSER_SMOKE_PORT || '4173');
const timeoutMs = Number(process.env.STELLAR_FORGE_BROWSER_SMOKE_TIMEOUT_MS || '30000');
const baseUrl = `http://${host}:${port}`;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function log(message) {
  console.log(`[ui-browser-smoke] ${message}`);
}

function wait(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

function detectPackageManager() {
  const explicit = process.env.STELLAR_FORGE_PACKAGE_MANAGER;
  if (explicit && explicit.length > 0) {
    return explicit;
  }
  const userAgent = process.env.npm_config_user_agent || '';
  if (userAgent.startsWith('pnpm/')) {
    return 'pnpm';
  }
  if (userAgent.startsWith('bun/')) {
    return 'bun';
  }
  if (userAgent.startsWith('yarn/')) {
    return 'yarn';
  }
  if (userAgent.startsWith('npm/')) {
    return 'npm';
  }
  return 'pnpm';
}

function browserInstallArgs() {
  const args = ['install'];
  if (process.platform === 'linux' && process.env.CI) {
    args.push('--with-deps');
  }
  args.push('chromium');
  return args;
}

function playwrightInvoker(packageManager) {
  switch (packageManager) {
    case 'pnpm':
      return { command: 'pnpm', prefix: ['dlx', playwrightVersion] };
    case 'bun':
      return { command: 'bunx', prefix: [playwrightVersion] };
    default:
      return { command: 'npx', prefix: ['--yes', playwrightVersion] };
  }
}

function run(command, args, cwd = appRoot) {
  const rendered = [command, ...args].join(' ');
  log(`running: ${rendered}`);
  const result = spawnSync(command, args, {
    cwd,
    env: {
      ...process.env,
      COREPACK_ENABLE_AUTO_PIN: process.env.COREPACK_ENABLE_AUTO_PIN || '0',
    },
    stdio: 'inherit',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${rendered} exited with status ${result.status ?? 1}`);
  }
}

function runCapture(command, args, cwd = appRoot) {
  const rendered = [command, ...args].join(' ');
  log(`running: ${rendered}`);
  const result = spawnSync(command, args, {
    cwd,
    env: {
      ...process.env,
      COREPACK_ENABLE_AUTO_PIN: process.env.COREPACK_ENABLE_AUTO_PIN || '0',
    },
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${rendered} exited with status ${result.status ?? 1}`);
  }
  return {
    stdout: result.stdout || '',
    stderr: result.stderr || '',
  };
}

function spawnPreview() {
  const args = ['preview', '--host', host, '--port', String(port), '--strictPort'];
  const child = spawn(process.execPath, [viteBin, ...args], {
    cwd: appRoot,
    env: {
      ...process.env,
      COREPACK_ENABLE_AUTO_PIN: process.env.COREPACK_ENABLE_AUTO_PIN || '0',
    },
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
      return { stdout, stderr };
    },
    rendered() {
      return [process.execPath, viteBin, ...args].join(' ');
    },
  };
}

async function waitForServer(preview) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (preview.child.exitCode !== null) {
      const output = preview.output();
      throw new Error(
        `preview server exited early\n${output.stderr || output.stdout || 'no output'}`,
      );
    }
    try {
      const response = await fetch(baseUrl);
      if (response.ok) {
        return;
      }
    } catch (_error) {
      // The preview server may still be booting.
    }
    await wait(250);
  }

  const output = preview.output();
  throw new Error(`timed out waiting for ${baseUrl}\n${output.stderr || output.stdout || 'no output'}`);
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

function chromiumInstallSignature() {
  return [
    `chromium-${chromiumRevision}`,
    `chromium_headless_shell-${chromiumRevision}`,
  ];
}

function listInstalledBrowsers() {
  const invoker = playwrightInvoker(detectPackageManager());
  const output = runCapture(invoker.command, [...invoker.prefix, 'install', '--list', 'chromium']);
  return `${output.stdout}\n${output.stderr}`;
}

function chromiumAlreadyInstalled() {
  const output = listInstalledBrowsers();
  return chromiumInstallSignature().every((signature) => output.includes(signature));
}

function ensureChromiumInstalled() {
  if (chromiumAlreadyInstalled()) {
    log(`Chromium revision ${chromiumRevision} already present; skipping install`);
    return;
  }
  const invoker = playwrightInvoker(detectPackageManager());
  log('installing Chromium for browser smoke');
  run(invoker.command, [...invoker.prefix, ...browserInstallArgs()]);
}

function assertChromiumInstalled() {
  assert(
    chromiumAlreadyInstalled(),
    'Chromium is not installed yet; run the browser smoke install step first',
  );
}

function writePlaywrightFiles(outputDir) {
  mkdirSync(outputDir, { recursive: true });
  const configPath = resolve(outputDir, 'playwright.config.cjs');
  const specPath = resolve(outputDir, 'frontend-browser-smoke.spec.cjs');
  const generatedState = readFileSync(resolve(appRoot, 'src', 'generated', 'stellar.ts'), 'utf8');

  writeFileSync(
    configPath,
    `const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: ${JSON.stringify(outputDir)},
  timeout: ${Number(timeoutMs)},
  workers: 1,
  reporter: 'line',
  use: {
    baseURL: ${JSON.stringify(baseUrl)},
    headless: true,
    screenshot: 'only-on-failure',
  },
});
`,
  );

  writeFileSync(
    specPath,
    `const { test, expect } = require('@playwright/test');

const generatedState = ${JSON.stringify(generatedState)};
const projectName = generatedState.match(/"name":\\s*"([^"]+)"/)?.[1] || 'demo';
const expectedSections = ['Queue', 'Runtime', 'Events', 'Contracts', 'Tokens'];

function isIgnorableRuntimeNoise(message) {
  return (
    message.includes('[DEP0169] DeprecationWarning') ||
    message.includes('url.parse() behavior is not standardized')
  );
}

test('generated frontend renders expected sections without browser errors', async ({ page }) => {
  const pageErrors = [];
  const consoleErrors = [];
  const failedResponses = [];

  page.on('pageerror', (error) => {
    const text = String(error);
    if (!isIgnorableRuntimeNoise(text)) {
      pageErrors.push(text);
    }
  });
  page.on('console', (message) => {
    const text = message.text();
    if (message.type() === 'error' && !isIgnorableRuntimeNoise(text)) {
      consoleErrors.push(text);
    }
  });
  page.on('response', (response) => {
    if (response.status() >= 400) {
      failedResponses.push(\`\${response.status()} \${response.url()}\`);
    }
  });

  await page.goto('/');
  await expect(page.locator('h1')).toContainText(projectName);
  await expect(page.locator('main')).toContainText('RPC');
  await expect(page.getByText(/stellar forge release (deploy|verify)/)).toBeVisible();

  for (const section of expectedSections) {
    await expect(page.getByText(section, { exact: true }).first()).toBeVisible();
  }

  expect(pageErrors, \`page errors:\\n\${pageErrors.join('\\n')}\`).toEqual([]);
  expect(consoleErrors, \`console errors:\\n\${consoleErrors.join('\\n')}\`).toEqual([]);
  expect(
    failedResponses,
    \`failed network responses:\\n\${failedResponses.join('\\n')}\`,
  ).toEqual([]);
});
`,
  );

  return { configPath, specPath };
}

function buildFrontendBundle() {
  const viteBin = resolve(appRoot, 'node_modules', 'vite', 'bin', 'vite.js');
  assert(
    existsSync(viteBin),
    'missing local Vite binary; run your package manager install command in apps/web first',
  );
  assert(
    existsSync(resolve(appRoot, 'src', 'generated', 'stellar.ts')),
    'missing generated frontend state file at src/generated/stellar.ts',
  );
  log('building frontend bundle');
  run(process.execPath, [viteBin, 'build']);
}

async function runBrowserSmoke() {
  assert(
    existsSync(resolve(appRoot, 'dist', 'index.html')),
    'missing built frontend bundle at dist/index.html; run the browser smoke build step first',
  );
  const tempRoot = mkdtempSync(resolve(os.tmpdir(), 'stellar-forge-ui-browser-smoke-'));
  const playwrightRoot = resolve(tempRoot, '.playwright');
  const packageManager = detectPackageManager();
  const invoker = playwrightInvoker(packageManager);
  const preview = spawnPreview();

  try {
    const { configPath, specPath } = writePlaywrightFiles(playwrightRoot);
    log(`starting preview server with ${preview.rendered()}`);
    await waitForServer(preview);
    log('running browser smoke test');
    run(invoker.command, [...invoker.prefix, 'test', specPath, '--config', configPath]);
    log('browser smoke passed');
  } finally {
    await stopProcess(preview.child);
    cleanup(tempRoot);
  }
}

function cleanup(tempRoot) {
  if (keepTemp) {
    log(`keeping temp directory at ${tempRoot}`);
    return;
  }
  rmSync(tempRoot, { recursive: true, force: true });
}

async function main() {
  const mode = process.argv[2] || 'all';

  if (mode === 'install') {
    ensureChromiumInstalled();
    return;
  }

  if (mode === 'build') {
    buildFrontendBundle();
    return;
  }

  if (mode === 'run') {
    assertChromiumInstalled();
    await runBrowserSmoke();
    return;
  }

  if (mode !== 'all') {
    throw new Error(
      `unknown browser smoke mode ${JSON.stringify(mode)}; expected all, install, build, or run`,
    );
  }

  buildFrontendBundle();
  ensureChromiumInstalled();
  await runBrowserSmoke();
}

main().catch((error) => {
  console.error(`[ui-browser-smoke] ${error.message}`);
  process.exit(1);
});

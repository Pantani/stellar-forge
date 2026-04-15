#!/usr/bin/env node
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import net from 'node:net';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const keepTemp = process.env.STELLAR_FORGE_BROWSER_SMOKE_KEEP === '1';

function log(message) {
  console.log(`[generated-browser-smoke] ${message}`);
}

function fail(message) {
  console.error(`[generated-browser-smoke] ${message}`);
  process.exit(1);
}

function resolveForgeBinary() {
  if (process.env.STELLAR_FORGE_BIN) {
    return process.env.STELLAR_FORGE_BIN;
  }

  const candidates = [
    path.join(repoRoot, 'target', 'debug', 'stellar-forge'),
    path.join(repoRoot, 'target', 'release', 'stellar-forge'),
  ];
  const existing = candidates.find((candidate) => existsSync(candidate));
  return existing || 'stellar-forge';
}

function run(command, args, cwd, extraEnv = {}) {
  const rendered = [command, ...args].join(' ');
  log(`running: ${rendered}`);
  const result = spawnSync(command, args, {
    cwd,
    env: {
      ...process.env,
      ...extraEnv,
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

function resolvePackageManager(projectRoot) {
  const explicit = process.env.STELLAR_FORGE_PACKAGE_MANAGER;
  if (explicit && explicit.length > 0) {
    return explicit;
  }

  const manifestPath = path.join(projectRoot, 'stellarforge.toml');
  if (!existsSync(manifestPath)) {
    return 'pnpm';
  }

  const manifest = readFileSync(manifestPath, 'utf8');
  return manifest.match(/^\s*package_manager\s*=\s*"([^"]+)"/m)?.[1] || 'pnpm';
}

function packageManagerInstallCommand(packageManager) {
  switch (packageManager) {
    case 'npm':
      return { command: 'npm', args: ['install', '--prefer-offline'] };
    case 'yarn':
      return { command: 'yarn', args: ['install'] };
    case 'bun':
      return { command: 'bun', args: ['install'] };
    default:
      return { command: 'pnpm', args: ['install', '--prefer-offline', '--reporter=silent'] };
  }
}

function packageManagerScriptCommand(packageManager, script) {
  switch (packageManager) {
    case 'npm':
      return { command: 'npm', args: ['run', script] };
    case 'bun':
      return { command: 'bun', args: ['run', script] };
    default:
      return { command: packageManager, args: [script] };
  }
}

function findAvailablePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        server.close(() => reject(new Error('could not resolve an ephemeral port')));
        return;
      }
      const { port } = address;
      server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(port);
      });
    });
  });
}

function cleanup(tempRoot) {
  if (keepTemp) {
    log(`keeping temp directory at ${tempRoot}`);
    return;
  }
  rmSync(tempRoot, { recursive: true, force: true });
}

async function main() {
  const forgeBinary = resolveForgeBinary();
  if (forgeBinary !== 'stellar-forge' && !existsSync(forgeBinary)) {
    fail(`could not find stellar-forge binary at ${forgeBinary}`);
  }

  const tempRoot = mkdtempSync(path.join(os.tmpdir(), 'stellar-forge-browser-smoke-'));
  const projectRoot = path.join(tempRoot, 'demo');

  try {
    run(forgeBinary, ['init', 'demo', '--template', 'fullstack'], tempRoot);
    const webRoot = path.join(projectRoot, 'apps', 'web');
    const packageManager = resolvePackageManager(projectRoot);
    const browserSmokePort = String(await findAvailablePort());
    log(`using package manager ${packageManager}`);
    log(`using browser smoke port ${browserSmokePort}`);
    const install = packageManagerInstallCommand(packageManager);
    run(install.command, install.args, webRoot);
    for (const script of ['smoke:browser:build', 'smoke:browser:install', 'smoke:browser:run']) {
      const command = packageManagerScriptCommand(packageManager, script);
      run(command.command, command.args, webRoot, {
        STELLAR_FORGE_BROWSER_SMOKE_PORT: browserSmokePort,
      });
    }
    log('generated frontend browser smoke passed');
  } finally {
    cleanup(tempRoot);
  }
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

function signingIdentity() {
  const keychain = join(homedir(), 'Library/Keychains/mtool-development.keychain-db');
  if (!existsSync(keychain)) return undefined;
  for (const args of [
    ['unlock-keychain', '-p', '', keychain],
    ['find-identity', '-v', '-p', 'codesigning', keychain],
  ]) {
    const result = spawnSync('/usr/bin/security', args, { encoding: 'utf8' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(result.stderr.trim() || '无法读取本地开发签名');
    if (args[0] === 'find-identity') {
      const identity = result.stdout.match(/\b([A-F0-9]{40})\s+"MTool Local Development"/i)?.[1];
      if (!identity) throw new Error('本地开发签名无效，请检查开发钥匙串');
      return identity;
    }
  }
}

try {
  const environment = { ...process.env };
  if (
    process.platform === 'darwin' &&
    !environment.APPLE_SIGNING_IDENTITY &&
    !environment.APPLE_CERTIFICATE
  ) {
    const identity = signingIdentity();
    if (identity) environment.APPLE_SIGNING_IDENTITY = identity;
  }
  const pnpm = process.env.npm_execpath;
  if (!pnpm) throw new Error('请通过 pnpm package 执行打包');
  const arguments_ = ['tauri', 'build', ...process.argv.slice(2)];
  const nodeScript = /\.(?:c|m)?js$/i.test(pnpm);
  const result = spawnSync(
    nodeScript ? process.execPath : pnpm,
    nodeScript ? [pnpm, ...arguments_] : arguments_,
    { cwd: fileURLToPath(new URL('..', import.meta.url)), stdio: 'inherit', env: environment },
  );
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}

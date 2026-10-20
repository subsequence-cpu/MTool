import { spawnSync } from 'node:child_process';
import { mkdtempSync, existsSync, rmSync, chmodSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command}: ${result.stderr.trim() || '执行失败'}`);
  return result.stdout;
}

function main() {
  if (process.platform !== 'darwin') throw new Error('仅支持 macOS');
  const keychain = join(homedir(), 'Library/Keychains/mtool-development.keychain-db');
  if (existsSync(keychain)) throw new Error('开发钥匙串已存在，不覆盖；请运行 pnpm package');
  const originalKeychains = [
    ...run('/usr/bin/security', ['list-keychains', '-d', 'user']).matchAll(/"([^"\n]+)"/g),
  ].map((match) => match[1]);
  const previousMask = process.umask(0o077);
  const directory = mkdtempSync(join(tmpdir(), 'mtool-signing-'));
  let createdKeychain = false;
  let installed = false;
  const certificate = join(directory, 'certificate.pem');
  const privateKey = join(directory, 'private-key.pem');
  try {
    run('openssl', [
      'req',
      '-x509',
      '-newkey',
      'rsa:3072',
      '-sha256',
      '-nodes',
      '-keyout',
      privateKey,
      '-out',
      certificate,
      '-days',
      '3650',
      '-subj',
      '/CN=MTool Local Development/',
      '-addext',
      'basicConstraints=critical,CA:TRUE',
      '-addext',
      'keyUsage=critical,digitalSignature,keyCertSign',
      '-addext',
      'extendedKeyUsage=codeSigning',
    ]);
    run('/usr/bin/security', ['create-keychain', '-p', '', keychain]);
    createdKeychain = true;
    chmodSync(keychain, 0o600);
    run('/usr/bin/security', ['import', certificate, '-k', keychain]);
    run('/usr/bin/security', ['import', privateKey, '-k', keychain, '-T', '/usr/bin/codesign']);
    run('/usr/bin/security', [
      'set-key-partition-list',
      '-S',
      'apple-tool:,apple:',
      '-s',
      '-k',
      '',
      keychain,
    ]);
    run('/usr/bin/security', [
      'add-trusted-cert',
      '-r',
      'trustRoot',
      '-p',
      'codeSign',
      '-k',
      keychain,
      certificate,
    ]);
    run('/usr/bin/security', [
      'list-keychains',
      '-d',
      'user',
      '-s',
      ...new Set([...originalKeychains, keychain]),
    ]);
    const identities = run('/usr/bin/security', [
      'find-identity',
      '-v',
      '-p',
      'codesigning',
      keychain,
    ]);
    if (!/"MTool Local Development"/.test(identities)) throw new Error('未找到有效的开发签名身份');
    installed = true;
    console.log('MTool 本地开发签名已配置。后续使用 pnpm package 打包。');
  } finally {
    if (createdKeychain && !installed) {
      spawnSync('/usr/bin/security', ['remove-trusted-cert', certificate]);
      spawnSync('/usr/bin/security', ['delete-keychain', keychain]);
      spawnSync('/usr/bin/security', ['list-keychains', '-d', 'user', '-s', ...originalKeychains]);
    }
    rmSync(directory, { recursive: true, force: true });
    process.umask(previousMask);
  }
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}

import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const cwd = fileURLToPath(new URL('..', import.meta.url));
for (const [name, example] of [
  ['mtool-core', 'export_bindings'],
  ['mtool-tools', 'export_catalog'],
]) {
  const result = spawnSync('cargo', ['run', '--release', '-p', name, '--example', example], {
    cwd,
    stdio: 'inherit',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

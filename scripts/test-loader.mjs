import { existsSync, statSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

const PROJECT_ROOT = resolveRoot();

function resolveRoot() {
  let dir = dirname(fileURLToPath(import.meta.url));
  while (dir !== '/') {
    if (existsSync(join(dir, 'src'))) {
      return dir;
    }
    dir = dirname(dir);
  }
  return process.cwd();
}

export async function resolve(specifier, context, nextResolve) {
  if (specifier.startsWith('@/')) {
    const relative = specifier.slice(2);
    const candidates = [
      join(PROJECT_ROOT, 'src', relative),
      join(PROJECT_ROOT, 'src', `${relative}.ts`),
      join(PROJECT_ROOT, 'src', `${relative}.tsx`),
      join(PROJECT_ROOT, 'src', relative, 'index.ts'),
    ];
    for (const candidate of candidates) {
      if (existsSync(candidate) && statSync(candidate).isFile()) {
        return nextResolve(pathToFileURL(candidate).href, context);
      }
    }
  }
  return nextResolve(specifier, context);
}

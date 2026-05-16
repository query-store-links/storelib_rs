#!/usr/bin/env node
// Assemble the per-target wasm-pack outputs (pkg/nodejs, pkg/web, pkg/bundler)
// into a single universal npm package at pkg/ with conditional `exports` so the
// same package name works in Node, browsers, and bundlers.

import { promises as fs } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..');
const pkgRoot = process.argv[2] ? join(repoRoot, process.argv[2]) : join(repoRoot, 'pkg');

const targets = ['nodejs', 'web', 'bundler'];

for (const t of targets) {
    const pj = join(pkgRoot, t, 'package.json');
    try { await fs.access(pj); }
    catch {
        console.error(`Missing build output for target '${t}' at ${pj}.`);
        console.error('Run tools/pack-npm.sh (or .ps1) first to build all targets.');
        process.exit(1);
    }
}

const base = JSON.parse(
    await fs.readFile(join(pkgRoot, 'bundler', 'package.json'), 'utf8'),
);

for (const k of ['main', 'module', 'types', 'files', 'type', 'sideEffects']) {
    delete base[k];
}

const merged = {
    ...base,
    main: './nodejs/storelib_rs.js',
    types: './bundler/storelib_rs.d.ts',
    exports: {
        '.': {
            types: './bundler/storelib_rs.d.ts',
            node: {
                types: './nodejs/storelib_rs.d.ts',
                default: './nodejs/storelib_rs.js',
            },
            import: './bundler/storelib_rs.js',
            default: './web/storelib_rs.js',
        },
        './package.json': './package.json',
    },
    files: ['nodejs', 'web', 'bundler', 'README.md', 'LICENSE'],
    sideEffects: [
        './bundler/storelib_rs.js',
        './web/storelib_rs.js',
    ],
};

// npm wants `repository.url` in `git+https://…/foo.git` form. wasm-pack
// emits the bare `repository = "https://github.com/owner/repo"` from
// Cargo.toml, which makes `npm publish` print a warning and auto-rewrite
// the field. Do the rewrite here so the published manifest is clean.
function normalizeRepoUrl(url) {
    if (!url || typeof url !== 'string') return url;
    if (url.startsWith('git+')) return url;
    if (url.startsWith('http://') || url.startsWith('https://')) {
        return 'git+' + (url.endsWith('.git') ? url : url + '.git');
    }
    return url;
}
if (typeof merged.repository === 'string') {
    merged.repository = { type: 'git', url: normalizeRepoUrl(merged.repository) };
} else if (merged.repository && typeof merged.repository === 'object') {
    merged.repository.url = normalizeRepoUrl(merged.repository.url);
    if (!merged.repository.type) merged.repository.type = 'git';
}

await fs.writeFile(
    join(pkgRoot, 'package.json'),
    JSON.stringify(merged, null, 2) + '\n',
);

// Empty .npmignore short-circuits npm's fallback to the repo .gitignore (which
// usually excludes /pkg from version control).
await fs.writeFile(join(pkgRoot, '.npmignore'), '');

for (const f of ['README.md', 'LICENSE']) {
    try { await fs.copyFile(join(pkgRoot, 'bundler', f), join(pkgRoot, f)); }
    catch { /* optional file */ }
}

const typeMarker = { nodejs: 'commonjs', web: 'module', bundler: 'module' };
for (const t of targets) {
    await fs.writeFile(
        join(pkgRoot, t, 'package.json'),
        JSON.stringify({ type: typeMarker[t] }, null, 2) + '\n',
    );
    for (const entry of await fs.readdir(join(pkgRoot, t))) {
        // wasm-pack writes `.gitignore` with `*` so users don't accidentally
        // commit build outputs. npm pack honors per-directory .gitignore, so we
        // must remove it (or none of the target files will be in the tarball).
        if (
            entry === 'README.md' ||
            entry === 'LICENSE' ||
            entry === '.gitignore' ||
            entry.endsWith('.tgz')
        ) {
            await fs.unlink(join(pkgRoot, t, entry));
        }
    }
}

console.log(`Assembled universal package at ${pkgRoot}`);
console.log(JSON.stringify(merged, null, 2));

#!/usr/bin/env node
// Generate a TypeScript .d.ts containing every serializable Rust type that
// crosses the wasm boundary, derived from the Rust source. Designed to be run
// by `build.rs` (writes to the path passed as argv[2]) but also usable
// standalone (no arg → prints to stdout).
//
// What gets emitted:
//   * String-union aliases for every `#[derive(Serialize)]` enum (camelCase
//     variant names follow serde's `rename_all`).
//   * `interface` declarations for every `#[derive(Serialize)]` struct, with
//     rustdoc forwarded to JSDoc. Field names follow `rename_all`/`rename`.
//   * A `ProgressStage` union scraped from `.emit("…")` / `stage: "…"` calls.
//   * A `StorelibError` interface derived from the `store_err` mapping in
//     wasm.rs (which converts `StoreError` variants into JS `kind` strings).
//   * Type aliases for the iso-code enums (`Market`, `Lang`, `LanguageTag`)
//     that are too large to enumerate and serialize as `string` anyway.
//
// Field-type mapping:
//   String/&str/&'static str    → string
//   bool                        → boolean
//   i8…i32/u8…u32/f32/f64       → number
//   i64/u64                     → number | bigint  (BigInt for safe-int overflow)
//   Option<T>                   → T | null
//   Vec<T>                      → T[]
//   serde_json::Value           → any
//   #[serde(default)] field     → optional (`?: T`)
//   /// @ts-type X              → field overridden to `X`
//
// Re-run after touching any of:
//   src/models/{enums,fe3,locale,search,catalog}.rs
//   src/services/display_catalog.rs
//   src/wasm.rs   src/error.rs

import fs from 'node:fs/promises';
import { dirname, join, resolve as resolvePath } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const RUST_SOURCES = [
    'src/models/enums.rs',
    'src/models/fe3.rs',
    'src/models/locale.rs',
    'src/models/search.rs',
    'src/models/catalog.rs',
    'src/services/display_catalog.rs',
    'src/wasm.rs',
];
const STAGE_SOURCES = ['src/services/display_catalog.rs'];
const ERROR_KIND_SOURCE = 'src/wasm.rs';

// Some Rust type names collide with wasm-bindgen class exports of the same name
// — the JSON interface gets a `Json` suffix to keep TS happy.
const TYPE_RENAMES = {
    Locale: 'LocaleJson',
};

// Enums in iso_codes.rs are too large (~250 variants) to inline as TS unions
// and serialize as plain strings anyway; expose a `string` alias instead.
const ISO_CODE_TYPES = new Set(['Market', 'Lang', 'LanguageTag']);
const ISO_CODE_DOC = {
    Market: 'Two-letter ISO 3166-1 alpha-2 market code (e.g. "US", "JP").',
    Lang: 'Two-letter ISO 639-1 language code (e.g. "en", "zh").',
    LanguageTag: 'Microsoft Store BCP-47 language tag (e.g. "en-US", "zh-Hant-TW").',
};

// ---------------------------------------------------------------------------
// Casing
// ---------------------------------------------------------------------------

const pascalToCamel = s => (s ? s[0].toLowerCase() + s.slice(1) : s);
const snakeToCamel  = s => s.replace(/_([a-zA-Z0-9])/g, (_, c) => c.toUpperCase());

function renameField(name, rule) {
    if (!rule) return name;
    switch (rule) {
        case 'camelCase':              return snakeToCamel(name);
        case 'PascalCase':             return name.split('_').map(p => p[0].toUpperCase() + p.slice(1)).join('');
        case 'snake_case':             return name;
        case 'SCREAMING_SNAKE_CASE':   return name.toUpperCase();
        case 'kebab-case':             return name.replace(/_/g, '-');
        case 'SCREAMING-KEBAB-CASE':   return name.replace(/_/g, '-').toUpperCase();
        case 'lowercase':              return name.toLowerCase();
        case 'UPPERCASE':              return name.toUpperCase();
        default:                       return name;
    }
}

function renameVariant(name, rule) {
    if (!rule) return name;
    switch (rule) {
        case 'camelCase':              return pascalToCamel(name);
        case 'PascalCase':             return name;
        case 'snake_case':             return name.replace(/(?<!^)([A-Z])/g, '_$1').toLowerCase();
        case 'SCREAMING_SNAKE_CASE':   return name.replace(/(?<!^)([A-Z])/g, '_$1').toUpperCase();
        case 'kebab-case':             return name.replace(/(?<!^)([A-Z])/g, '-$1').toLowerCase();
        case 'SCREAMING-KEBAB-CASE':   return name.replace(/(?<!^)([A-Z])/g, '-$1').toUpperCase();
        case 'lowercase':              return name.toLowerCase();
        case 'UPPERCASE':              return name.toUpperCase();
        default:                       return name;
    }
}

// ---------------------------------------------------------------------------
// Rust → TS type mapping
// ---------------------------------------------------------------------------

function mapRustType(rust) {
    let t = rust.trim();

    // Strip a leading reference + lifetime + mut.
    const refStripped = t.replace(/^&(?:'\w+\s+)?(?:mut\s+)?/, '');
    if (refStripped !== t) return mapRustType(refStripped);

    // Strip path qualifiers: `crate::models::catalog::Product` → `Product`.
    const pathM = t.match(/^(?:[A-Za-z_]\w*::)+([A-Za-z_]\w*(?:\s*<[\s\S]*>)?)$/);
    if (pathM) t = pathM[1];

    // Option<T> → T | null
    const optM = t.match(/^Option\s*<\s*([\s\S]+)\s*>$/);
    if (optM) {
        const inner = mapRustType(optM[1]);
        return `${inner} | null`;
    }

    // Vec<T>, &[T], [T; N] → T[]
    const vecM = t.match(/^(?:Vec|VecDeque|HashSet|BTreeSet)\s*<\s*([\s\S]+)\s*>$/);
    if (vecM) {
        const inner = mapRustType(vecM[1]);
        const needsParens = /[ |&]/.test(inner) && !/^\(.*\)$/.test(inner);
        return needsParens ? `(${inner})[]` : `${inner}[]`;
    }

    // Box<T>, Rc<T>, Arc<T> → T (transparent wrappers)
    const boxM = t.match(/^(?:Box|Rc|Arc|Cow)\s*<\s*(?:'\w+\s*,\s*)?([\s\S]+)\s*>$/);
    if (boxM) return mapRustType(boxM[1]);

    // HashMap<K, V> / BTreeMap<K, V>
    const mapM = t.match(/^(?:HashMap|BTreeMap)\s*<\s*([^,]+)\s*,\s*([\s\S]+)\s*>$/);
    if (mapM) {
        const v = mapRustType(mapM[2]);
        // TS index signatures only allow `string`/`number`/`symbol` keys; force string.
        return `{ [k: string]: ${v} }`;
    }

    // Tuple
    const tupleM = t.match(/^\(\s*([\s\S]*)\s*\)$/);
    if (tupleM && tupleM[1].trim().length > 0) {
        const parts = splitTopLevelCommas(tupleM[1]).map(mapRustType);
        return `[${parts.join(', ')}]`;
    }

    // Primitives
    switch (t) {
        case 'String': case 'str':                                          return 'string';
        case 'bool':                                                        return 'boolean';
        case 'i8': case 'i16': case 'i32': case 'u8': case 'u16': case 'u32':
        case 'isize': case 'usize': case 'f32': case 'f64':                 return 'number';
        case 'i64': case 'u64': case 'i128': case 'u128':                   return 'number | bigint';
        case 'serde_json::Value': case 'Value':                             return 'any';
        case 'Uuid':                                                        return 'string';
        case '()':                                                          return 'void';
    }

    // Named type — apply rename map, else pass through.
    const baseM = t.match(/^([A-Z]\w*)\s*(?:<([\s\S]*)>)?$/);
    if (baseM) {
        const name = baseM[1];
        // For now, generic args on named types are dropped (none of our
        // serialized types use generics in practice).
        return TYPE_RENAMES[name] || name;
    }

    return 'any';
}

function splitTopLevelCommas(s) {
    const out = [];
    let depth = 0, start = 0;
    for (let i = 0; i < s.length; i++) {
        const c = s[i];
        if (c === '<' || c === '(' || c === '[') depth++;
        else if (c === '>' || c === ')' || c === ']') depth--;
        else if (c === ',' && depth === 0) {
            out.push(s.slice(start, i).trim());
            start = i + 1;
        }
    }
    out.push(s.slice(start).trim());
    return out;
}

// ---------------------------------------------------------------------------
// Rust source parser
// ---------------------------------------------------------------------------

function collectAttributeOrDoc(lines, i, attrs, docs) {
    const raw = lines[i];
    const trimmed = raw.trim();

    if (trimmed.startsWith('///')) {
        docs.push(trimmed.replace(/^\/\/\/\s?/, ''));
        return i + 1;
    }

    if (trimmed.startsWith('#[')) {
        // Attribute may span multiple lines; consume until brackets and parens balance.
        let buf = trimmed;
        let brackets = 0, parens = 0;
        const tally = (s) => {
            for (const c of s) {
                if      (c === '[') brackets++;
                else if (c === ']') brackets--;
                else if (c === '(') parens++;
                else if (c === ')') parens--;
            }
        };
        tally(buf);
        while ((brackets > 0 || parens > 0) && i + 1 < lines.length) {
            i++;
            buf += '\n' + lines[i];
            tally(lines[i]);
        }
        attrs.push(buf);
        return i + 1;
    }

    return null; // not a doc or attribute line
}

function captureBracedBody(lines, startIdx) {
    // Returns the body text starting at the line containing `{` through the
    // matching `}`, plus the line index *after* the closing brace.
    let depth = 0;
    let started = false;
    let body = '';
    let j = startIdx;
    while (j < lines.length) {
        const l = lines[j];
        let inStr = false, sq = false, delim = '';
        for (let k = 0; k < l.length; k++) {
            const c = l[k];
            if (inStr) {
                if (c === '\\') { k++; continue; }
                if (c === delim) inStr = false;
                continue;
            }
            if (c === '"') { inStr = true; delim = c; continue; }
            if (c === '/' && l[k + 1] === '/') break; // rest of line is a comment
            if (c === '{') { depth++; started = true; }
            else if (c === '}') depth--;
        }
        body += l + '\n';
        if (started && depth === 0) return { body, next: j + 1 };
        j++;
    }
    return { body, next: j };
}

function* iterateTopLevelItems(src) {
    const lines = src.split(/\r?\n/);
    let i = 0;
    let docs = [];
    let attrs = [];

    while (i < lines.length) {
        const raw = lines[i];
        const trimmed = raw.trim();
        const indent = raw.length - raw.trimStart().length;

        // Only collect docs/attrs from column-0 lines so we don't pick up
        // anything inside fn bodies.
        if (indent === 0 && (trimmed.startsWith('///') || trimmed.startsWith('#['))) {
            const next = collectAttributeOrDoc(lines, i, attrs, docs);
            if (next !== null) { i = next; continue; }
        }

        // Skip module-doc/comment/empty lines without dropping docs.
        if (trimmed === '' || trimmed.startsWith('//')) {
            // A blank line *between* a doc block and the item it documents
            // would break the association in rustdoc — drop accumulated docs/attrs
            // when we see one.
            if (trimmed === '' && (docs.length || attrs.length)) {
                docs = []; attrs = [];
            }
            i++; continue;
        }

        // Top-level struct/enum?
        if (indent === 0) {
            const m = trimmed.match(/^(?:pub(?:\([^)]+\))?\s+)?(struct|enum)\s+([A-Za-z_]\w*)\b/);
            if (m) {
                const [, kind, name] = m;
                if (trimmed.includes('{')) {
                    const { body, next } = captureBracedBody(lines, i);
                    yield { kind, name, body, docs, attrs };
                    docs = []; attrs = []; i = next; continue;
                }
                // Unit struct (`pub struct Foo;`) — nothing to emit.
                yield { kind, name, body: '', docs, attrs };
                docs = []; attrs = []; i++; continue;
            }
        }

        // Some other top-level construct (use/fn/impl/type/etc.) — discard
        // any pending docs/attrs, since they belonged to that, not to us.
        if (indent === 0) {
            docs = []; attrs = [];
        }
        i++;
    }
}

// ---------------------------------------------------------------------------
// Serde attribute parsing
// ---------------------------------------------------------------------------

function readSerdeInner(attr) {
    const body = attr.replace(/^#\s*\[\s*/, '').replace(/\s*\]\s*$/, '').trim();
    if (!/^serde\b/.test(body)) return null;
    const m = body.match(/serde\s*\(([\s\S]*)\)\s*$/);
    return m ? m[1] : null;
}

function parseContainerSerde(attrs) {
    let renameAll = null;
    const derives = new Set();
    for (const attr of attrs) {
        const body = attr.replace(/^#\s*\[\s*/, '').replace(/\s*\]\s*$/, '').trim();
        if (/^derive\b/.test(body)) {
            const m = body.match(/derive\s*\(([\s\S]*)\)/);
            if (m) {
                for (const piece of splitTopLevelCommas(m[1])) {
                    derives.add(piece.trim().replace(/^.*::/, ''));
                }
            }
        } else if (/^serde\b/.test(body)) {
            const inner = readSerdeInner(attr);
            if (!inner) continue;
            let m;
            if ((m = inner.match(/rename_all\s*\(\s*serialize\s*=\s*"([^"]+)"/))) renameAll = m[1];
            else if ((m = inner.match(/rename_all\s*=\s*"([^"]+)"/))) renameAll = m[1];
        }
    }
    return { renameAll, derives };
}

function parseMemberSerde(attrs) {
    let rename = null;
    let isDefault = false;
    let skip = false;
    for (const attr of attrs) {
        const inner = readSerdeInner(attr);
        if (!inner) continue;
        let m;
        if ((m = inner.match(/rename\s*\(\s*serialize\s*=\s*"([^"]+)"/))) rename = m[1];
        else if ((m = inner.match(/(?<![_a-zA-Z])rename\s*=\s*"([^"]+)"/))) rename = m[1];
        if (/(?:^|[(,\s])default(?:[,\s)]|$)/.test(inner)) isDefault = true;
        if (/skip_serializing(?![_a-z])/.test(inner)) skip = true;
    }
    return { rename, isDefault, skip };
}

// ---------------------------------------------------------------------------
// Body parsers
// ---------------------------------------------------------------------------

function unwrapBraces(body) {
    const first = body.indexOf('{');
    const last  = body.lastIndexOf('}');
    return first >= 0 && last > first ? body.slice(first + 1, last) : '';
}

function parseStructFields(body) {
    const inner = unwrapBraces(body);
    const lines = inner.split(/\r?\n/);
    const fields = [];
    let docs = [], attrs = [];
    let i = 0;

    while (i < lines.length) {
        const raw = lines[i];
        const trimmed = raw.trim();

        if (trimmed === '') { i++; continue; }
        if (trimmed.startsWith('//') && !trimmed.startsWith('///')) { i++; continue; }

        if (trimmed.startsWith('///')) {
            docs.push(trimmed.replace(/^\/\/\/\s?/, ''));
            i++; continue;
        }

        if (trimmed.startsWith('#[')) {
            const next = collectAttributeOrDoc(lines, i, attrs, docs);
            i = next ?? i + 1;
            continue;
        }

        // Field declaration: optional visibility, name, type, optional comma.
        // Capture the entire declaration even when generic arguments span lines.
        let decl = trimmed;
        const tally = (s, st) => {
            for (const c of s) {
                if (c === '<') st.angle++;
                else if (c === '>') st.angle--;
                else if (c === '(') st.paren++;
                else if (c === ')') st.paren--;
            }
            return st;
        };
        const st = { angle: 0, paren: 0 };
        tally(decl, st);
        while ((st.angle > 0 || st.paren > 0 || !decl.includes(':'))
            && i + 1 < lines.length) {
            i++;
            decl += ' ' + lines[i].trim();
            tally(lines[i], st);
        }

        // Strip trailing comma if any.
        const declClean = decl.replace(/,\s*$/, '').trim();
        // Field pattern: `pub(crate)? r#? name : Type`
        // The `r#` prefix is Rust's raw-identifier escape (`r#type`) — strip it.
        const fm = declClean.match(/^(?:pub(?:\([^)]+\))?\s+)?(?:r#)?([A-Za-z_]\w*)\s*:\s*([\s\S]+)$/);
        if (fm) fields.push({ name: fm[1], type: fm[2].trim(), docs, attrs });

        docs = []; attrs = []; i++;
    }

    return fields;
}

function parseEnumVariants(body) {
    const inner = unwrapBraces(body);
    const lines = inner.split(/\r?\n/);
    const variants = [];
    let docs = [], attrs = [];
    let i = 0;

    while (i < lines.length) {
        const raw = lines[i];
        const trimmed = raw.trim();

        if (trimmed === '') { i++; continue; }
        if (trimmed.startsWith('//') && !trimmed.startsWith('///')) { i++; continue; }

        if (trimmed.startsWith('///')) {
            docs.push(trimmed.replace(/^\/\/\/\s?/, ''));
            i++; continue;
        }

        if (trimmed.startsWith('#[')) {
            const next = collectAttributeOrDoc(lines, i, attrs, docs);
            i = next ?? i + 1;
            continue;
        }

        // Variant identifier with optional payload — we only support unit variants.
        const vm = trimmed.match(/^([A-Z]\w*)\s*(?:[,({]|$)/);
        if (vm) variants.push({ name: vm[1], docs, attrs });
        docs = []; attrs = []; i++;
    }

    return variants;
}

// ---------------------------------------------------------------------------
// JSDoc emit
// ---------------------------------------------------------------------------

function escapeJsdoc(s) {
    // Avoid prematurely closing the JSDoc block.
    return s.replace(/\*\//g, '*​/');
}

function formatJsdoc(docs, indent) {
    if (!docs.length) return '';
    const cleaned = docs.map(escapeJsdoc);
    if (cleaned.length === 1 && !cleaned[0].includes('\n')) {
        return `${indent}/** ${cleaned[0]} */\n`;
    }
    return `${indent}/**\n${cleaned.map(d => `${indent} * ${d}`).join('\n')}\n${indent} */\n`;
}

// ---------------------------------------------------------------------------
// Emitters
// ---------------------------------------------------------------------------

function emitStruct(item) {
    const meta = parseContainerSerde(item.attrs);
    if (!meta.derives.has('Serialize')) return null;

    const fields = parseStructFields(item.body);
    const tsName = TYPE_RENAMES[item.name] || item.name;
    const out = [];

    out.push(formatJsdoc(item.docs, '').trimEnd());
    out.push(`export interface ${tsName} {`);

    for (const f of fields) {
        const fm = parseMemberSerde(f.attrs);
        if (fm.skip) continue;

        const jsonName = fm.rename ?? renameField(f.name, meta.renameAll);
        const propName = /^[A-Za-z_$][\w$]*$/.test(jsonName) ? jsonName : `"${jsonName}"`;

        // /// @ts-type X overrides the derived TS type.
        let overrideType = null;
        const cleanDocs = [];
        for (const d of f.docs) {
            const om = d.match(/^@ts-type\s+(.+)$/);
            if (om) overrideType = om[1].trim();
            else cleanDocs.push(d);
        }
        const tsType = overrideType ?? mapRustType(f.type);
        const optional = fm.isDefault ? '?' : '';

        const docBlock = formatJsdoc(cleanDocs, '    ').trimEnd();
        if (docBlock) out.push(docBlock);
        out.push(`    ${propName}${optional}: ${tsType};`);
    }

    out.push('}');
    return out.filter(s => s.length > 0).join('\n');
}

function emitEnum(item) {
    const meta = parseContainerSerde(item.attrs);
    if (!meta.derives.has('Serialize')) return null;

    const variants = parseEnumVariants(item.body);
    const tsName = TYPE_RENAMES[item.name] || item.name;
    const rule = meta.renameAll || 'PascalCase';
    const union = variants
        .map(v => {
            const vm = parseMemberSerde(v.attrs);
            const name = vm.rename ?? renameVariant(v.name, rule);
            return `"${name}"`;
        })
        .join(' | ');

    const head = formatJsdoc(item.docs, '').trimEnd();
    return `${head ? head + '\n' : ''}export type ${tsName} = ${union};`;
}

// ---------------------------------------------------------------------------
// Derived (non-struct) types
// ---------------------------------------------------------------------------

function extractProgressStages(src) {
    const found = new Set();
    const patterns = [
        /\.emit(?:_counter)?\s*\(\s*"([a-z][\w.]*\.[a-zA-Z][\w.]*)"/g,
        /\bon_progress\s*\(\s*"([a-z][\w.]*\.[a-zA-Z][\w.]*)"/g,
        /\bstage\s*:\s*"([a-z][\w.]*\.[a-zA-Z][\w.]*)"/g,
    ];
    for (const re of patterns) {
        let m;
        while ((m = re.exec(src)) !== null) found.add(m[1]);
    }
    // Stable ordering: group by prefix (dcat, fe3, search, retry) then alpha.
    const order = { dcat: 0, fe3: 1, search: 2, retry: 3 };
    return [...found].sort((a, b) => {
        const pa = order[a.split('.', 1)[0]] ?? 99;
        const pb = order[b.split('.', 1)[0]] ?? 99;
        return pa - pb || a.localeCompare(b);
    });
}

function extractErrorKinds(src) {
    // Match `StoreError::Variant(...) => "kind"` from wasm.rs's `store_err`.
    const re = /StoreError\s*::\s*\w+(?:\s*\([^)]*\))?\s*=>\s*"([A-Za-z][\w]*)"/g;
    const kinds = new Set();
    let m;
    while ((m = re.exec(src)) !== null) kinds.add(m[1]);
    return [...kinds];
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function read(file) {
    return fs.readFile(join(repoRoot, file), 'utf8');
}

async function build() {
    const out = [];

    out.push('// Auto-generated by `tools/gen-ts.mjs` from the Rust source tree.');
    out.push('// Do not edit by hand — re-run the generator after changing any of:');
    out.push('//   src/models/{enums,fe3,locale,search,catalog}.rs');
    out.push('//   src/services/display_catalog.rs   src/wasm.rs   src/error.rs');
    out.push('');
    out.push('/* IMPORTANT — BigInt for large numbers');
    out.push(' * ------------------------------------');
    out.push(' * Microsoft Store occasionally returns 64-bit integers that exceed');
    out.push(' * `Number.MAX_SAFE_INTEGER` (2⁵³ − 1), e.g. `ratingCount`,');
    out.push(' * `purchaseCount`, `playCount`. To keep precision, every 64-bit');
    out.push(' * integer field is emitted as `number | bigint`. To send the value');
    out.push(' * back through `JSON.stringify`, install a BigInt-aware replacer:');
    out.push(' *     JSON.stringify(v, (_k, v) => typeof v === \'bigint\' ? v.toString() : v);');
    out.push(' */');
    out.push('');

    // ISO-code aliases (the underlying enums are too large to enumerate).
    for (const t of ISO_CODE_TYPES) {
        out.push(`/** ${ISO_CODE_DOC[t]} */`);
        out.push(`export type ${t} = string;`);
        out.push('');
    }

    // Walk every configured Rust source and emit each Serialize-derived item.
    const seen = new Set();
    for (const file of RUST_SOURCES) {
        const src = await read(file);
        for (const item of iterateTopLevelItems(src)) {
            if (ISO_CODE_TYPES.has(item.name)) continue;
            const tsName = TYPE_RENAMES[item.name] || item.name;
            if (seen.has(tsName)) continue;

            const rendered = item.kind === 'struct' ? emitStruct(item) : emitEnum(item);
            if (rendered) {
                seen.add(tsName);
                out.push(rendered);
                out.push('');
            }
        }
    }

    // ProgressStage union scraped from .emit() call sites.
    const stageSrc = (await Promise.all(STAGE_SOURCES.map(read))).join('\n');
    const stages = extractProgressStages(stageSrc);
    if (stages.length === 0) throw new Error('No progress stages discovered');
    out.push('/** Stable progress-event stage identifier (see `ProgressEvent.stage`).');
    out.push(' *  Auto-derived from the `.emit()` call sites in services. */');
    out.push('export type ProgressStage =');
    out.push('    | ' + stages.map(s => `"${s}"`).join('\n    | ') + ';');
    out.push('');

    // OnProgress callback alias — pure TS sugar.
    out.push('/** Progress callback installed via `DisplayCatalogHandler.onProgress`. */');
    out.push('export type OnProgress = (event: ProgressEvent) => void;');
    out.push('');

    // StorelibError — the JS-side shape, with `kind` derived from `store_err`.
    const errSrc = await read(ERROR_KIND_SOURCE);
    const kinds = extractErrorKinds(errSrc);
    if (kinds.length === 0) throw new Error('No StoreError kinds discovered');
    out.push('/** Error thrown by async handler methods. Branch on `.kind` to decide');
    out.push(' *  what to surface to the user.');
    out.push(' *');
    out.push(' *  `causes` is the Rust-side `source()` chain (excluding the top-level');
    out.push(' *  message, which lives in `.message`). The JS stack trace is on `.stack`');
    out.push(' *  as usual. */');
    out.push('export interface StorelibError extends Error {');
    out.push('    kind: ' + kinds.map(k => `"${k}"`).join(' | ') + ';');
    out.push('    causes: string[];');
    out.push('}');
    out.push('');

    return out.join('\n');
}

const ts = await build();
const arg = process.argv[2];
if (arg) {
    const dest = resolvePath(arg);
    await fs.mkdir(dirname(dest), { recursive: true });
    await fs.writeFile(dest, ts);
    console.error(`Wrote ${dest} (${ts.length} bytes)`);
} else {
    process.stdout.write(ts);
}

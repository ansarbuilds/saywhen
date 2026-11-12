#!/usr/bin/env node
// Check local Markdown links and heading anchors before docs go out.
// External URLs are not fetched here; a network check belongs in a separate job.
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const skipDirs = new Set(['node_modules', 'dist', 'target', '.git', 'recordings', 'models']);
const files = [];

function walk(dir) {
  for (const entry of readdirSync(dir)) {
    if (skipDirs.has(entry)) continue;
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) walk(full);
    else if (entry.endsWith('.md')) files.push(full);
  }
}

walk(root);

function slugify(text) {
  return text
    .toLowerCase()
    .replace(/<[^>]+>/g, '')
    .replace(/[`*_~]/g, '')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/[^\p{L}\p{N}\s-]/gu, '')
    .trim()
    .replace(/\s+/g, '-');
}

function headingsFor(file) {
  const text = readFileSync(file, 'utf8');
  const seen = new Map();
  const anchors = new Set();
  let fenced = false;
  for (const line of text.split('\n')) {
    if (/^\s*```/.test(line)) {
      fenced = !fenced;
      continue;
    }
    if (fenced) continue;
    const match = line.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (!match) continue;
    const base = slugify(match[2]);
    if (!base) continue;
    const count = seen.get(base) ?? 0;
    seen.set(base, count + 1);
    anchors.add(count === 0 ? base : `${base}-${count}`);
  }
  return anchors;
}

const anchorCache = new Map();
function anchorsFor(file) {
  if (!anchorCache.has(file)) anchorCache.set(file, headingsFor(file));
  return anchorCache.get(file);
}

const errors = [];
const linkPattern = /!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;

for (const file of files) {
  const text = readFileSync(file, 'utf8');
  let fenced = false;
  const lines = text.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^\s*```/.test(line)) {
      fenced = !fenced;
      continue;
    }
    if (fenced) continue;
    for (const match of line.matchAll(linkPattern)) {
      const raw = match[1];
      if (/^(https?:|mailto:|tel:|data:)/i.test(raw)) continue;
      const [pathPart, anchorPart] = raw.split('#');
      const anchor = anchorPart ? decodeURIComponent(anchorPart) : '';
      if (!pathPart) {
        if (anchor && !anchorsFor(file).has(anchor)) {
          errors.push(`${relative(root, file)}:${index + 1} missing anchor #${anchor}`);
        }
        continue;
      }
      const target = resolve(dirname(file), decodeURIComponent(pathPart));
      if (!existsSync(target)) {
        errors.push(`${relative(root, file)}:${index + 1} missing ${pathPart}`);
        continue;
      }
      if (anchor && target.endsWith('.md')) {
        if (!anchorsFor(target).has(anchor)) {
          errors.push(
            `${relative(root, file)}:${index + 1} missing anchor #${anchor} in ${relative(root, target)}`
          );
        }
      }
    }
  }
}

if (errors.length > 0) {
  console.error(`Docs check failed with ${errors.length} problem(s):`);
  for (const error of errors) console.error(`  ${error}`);
  process.exit(1);
}

console.log(`Docs check passed (${files.length} Markdown files).`);

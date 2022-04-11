#!/usr/bin/env -S deno run --allow-read --allow-write

// const path = require('deno:path');

const ALLOWED_FILETYPES = ['mp4', 'mkv'];
const RE_QUAL = /([0-9]{3,}p)/i;
const RE_SEASON = /(s[0-9]+e[0-9]+)/i;

const dir = Deno.args[0] || '.';

function capitalizeFirst(s: string): string {
  if (s.length === 0) return s;
  return s[0].toUpperCase() + s.substring(1);
}

function padDirectory(d: string): string {
  if (!d.endsWith('/')) d += '/';
  return d;
}

async function convertDirectory(directory: string, recursive = false) {
  const promises: Promise<unknown>[] = [];
  const d = padDirectory(directory);

  for await (const ent of Deno.readDir(directory)) {
    if (ent.isFile) {
      const fileSplit = ent.name.split(/[. -]+/);
      const fileType = fileSplit[fileSplit.length-1];
      let quality = 'UNKNOWNp';
      let episode = 'SXXEXX';
      if (!ALLOWED_FILETYPES.includes(fileType.toLowerCase())) continue;

      let nameEnd = fileSplit.length;
      for (let i=0;i<fileSplit.length-1;i++) {
        let match = fileSplit[i].match(RE_QUAL);
        if (match) {
          quality = match[1].toLowerCase();
          if (i < nameEnd) nameEnd = i;
        }
        match = fileSplit[i].match(RE_SEASON);
        if (match) {
          episode = match[1].toUpperCase();
          if (i < nameEnd) nameEnd = i;
        }
      }
      const newFileName = `${fileSplit.slice(0, nameEnd).map(p => capitalizeFirst(p)).join(' ')}-${episode}-${quality}.${fileType}`;
      promises.push(Deno.rename(d + ent.name, d + newFileName));
      console.log(`Renaming "${ent.name}" -> "${newFileName}"`);
    } else if (ent.isDirectory && recursive) {
      promises.push(convertDirectory(ent.name, recursive));
    }
  }
  await Promise.all(promises);
}

await convertDirectory(dir);


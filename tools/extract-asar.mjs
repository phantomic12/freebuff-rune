import fs from 'node:fs';
import path from 'node:path';

const [asarPath, outDir] = process.argv.slice(2);
const buf = fs.readFileSync(asarPath);
const headerSize = buf.readUInt32LE(4);
// locate the JSON header start and end robustly
const jsonStart = buf.indexOf('{"files"', 0, 'utf8');
// header payload region ends at 8 + headerSize
const regionEnd = 8 + headerSize;
const region = buf.subarray(jsonStart, regionEnd).toString('utf8');
const header = JSON.parse(region.slice(0, region.lastIndexOf('}') + 1));
const baseOffset = 8 + headerSize;

let count = 0;
function walk(node, dir) {
  for (const [name, meta] of Object.entries(node.files || {})) {
    const target = path.join(dir, name);
    if (meta.files) { fs.mkdirSync(target, { recursive: true }); walk(meta, target); }
    else if (meta.link || meta.unpacked) continue;
    else if (meta.offset !== undefined) {
      const start = baseOffset + Number(meta.offset);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, buf.subarray(start, start + Number(meta.size)));
      count++;
    }
  }
}
fs.mkdirSync(outDir, { recursive: true });
walk(header, outDir);
console.log(`extracted ${count} files -> ${outDir}`);

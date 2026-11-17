// Count (and show the first hit of) strings in a minified bundle. How the orchestrator's boot
// phase names, route list and SSE surface were located without running the app.
//
//   node tools/scan.mjs extracted/orchestrator.js text/event-stream /api/threads
import fs from 'node:fs';

const [, , file, ...words] = process.argv;
if (!file || words.length === 0) {
  console.error('usage: node tools/scan.mjs <file> <word…>');
  process.exit(2);
}
const text = fs.readFileSync(file, 'utf8');
console.log(`${file}: ${text.length} bytes\n`);
for (const word of words) {
  const count = text.split(word).length - 1;
  console.log(`${String(count).padStart(6)}  ${word}`);
  const at = text.indexOf(word);
  if (at >= 0) {
    const context = text.slice(Math.max(0, at - 160), at + 160).replace(/\s+/g, ' ');
    console.log(`        …${context}…\n`);
  }
}

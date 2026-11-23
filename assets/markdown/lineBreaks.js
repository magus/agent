// Chat replies often use single newlines for separate lines (lists of values, addresses, poems).
// CommonMark folds those into spaces, so mark them as hard breaks outside fenced code blocks.
// Block syntax (lists, tables, headings, quotes) is unaffected: a trailing hard break before a new
// block or at the end of a paragraph renders nothing.

const fence_pattern = /^ {0,3}(`{3,}|~{3,})/;

/** @param {string} source */
export function keepLineBreaks(source) {
  const line_list = source.split('\n');
  let fence = null;
  for (let index = 0; index < line_list.length - 1; index += 1) {
    const line = line_list[index];
    const opening = line.match(fence_pattern);
    if (fence) {
      if (opening && opening[1][0] === fence[0] && opening[1].length >= fence.length
        && line.trim() === opening[1]) fence = null;
      continue;
    }
    if (opening) {
      fence = opening[1];
      continue;
    }
    const next = line_list[index + 1];
    if (line.trim() === '' || next.trim() === '' || line.endsWith('  ') || line.endsWith('\\')) continue;
    if (next.match(fence_pattern)) continue;
    line_list[index] = `${line}  `;
  }
  return line_list.join('\n');
}

// Executed inside the production Conversation JSON web view, after syntax highlighting.
const code = document.querySelector('code');
const walker = document.createTreeWalker(code, NodeFilter.SHOW_TEXT);
let last;
while (walker.nextNode()) {
  if (walker.currentNode.textContent.trimEnd()) last = walker.currentNode;
}
const end = last.textContent.trimEnd().length;
const range = document.createRange();
range.setStart(last, end - 1);
range.setEnd(last, end);
const width = document.documentElement.clientWidth;
const before = range.getBoundingClientRect().right;
if (before <= width) throw Error('The fixture fits without scrolling');
window.scrollTo(before - width / 2, 0);
const after = range.getBoundingClientRect().right;
if (after > width || after < 0) throw Error(`Last character at ${after} in ${width}px viewport`);

window.scrollTo(0, 0);
const render = (source) => window.render_message({source, active: false, dark: false, font_size: 14});
for (const source of ['a\nb', 'a\r\nb', 'a  \nb', 'a\\\nb']) {
  render(source);
  const paragraph = document.querySelector('p');
  const lineHeight = parseFloat(getComputedStyle(paragraph).lineHeight);
  const actualHeight = paragraph.getBoundingClientRect().height;
  if (Math.abs(actualHeight - 2 * lineHeight) > 1) {
    throw Error(`${JSON.stringify(source)}: expected two lines, measured ${actualHeight / lineHeight}`);
  }
}
for (const source of ['    alpha\n    beta', '> ```\n> alpha\n> beta\n> ```']) {
  render(source);
  if (document.querySelector('code').textContent !== 'alpha\nbeta\n') throw Error('Code changed');
}
render('An `alpha\nbeta` span');
if (document.querySelector('code').textContent !== 'alpha beta') throw Error('Inline code changed');
return {before, after, width};

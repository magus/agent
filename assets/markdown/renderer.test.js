import { imageUrlOptions } from './imageUrlOptions.js';
import { keepLineBreaks } from './lineBreaks.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHtmlRenderer } from '@pierre/words/html';

test('renders the shared formatting fixture with semantic structure', () => {
  const fixture_url = new URL('./fixtures/markdown.md', import.meta.url);
  const source = readFileSync(fixture_url, 'utf8');
  const renderer = createHtmlRenderer({ controls: false, lineNumbers: false });

  try {
    const html = renderer.render(source, false).html;
    for (const tag of ['h1', 'p', 'strong', 'a', 'ul', 'ol', 'blockquote', 'pre', 'code', 'table', 'th', 'td']) {
      assert.match(html, new RegExp(`<${tag}[ >]`));
    }
    assert.match(html, /href="https:\/\/example.com"/);
    assert.match(html, /inline_code/);
    assert.match(html, /Nested item/);
    assert.match(html, /Ordered second/);
    assert.match(html, /print\(value\)/);
    assert.equal((html.match(/<ul[ >]/g) ?? []).length, 2);
    assert.equal((html.match(/<th[ >]/g) ?? []).length, 2);
    assert.equal((html.match(/<td[ >]/g) ?? []).length, 2);
  } finally {
    renderer.dispose();
  }
});

test('untrusted markdown cannot insert executable HTML or load remote images', () => {
  const renderer = createHtmlRenderer({ ...imageUrlOptions([]), controls: false });
  const source = '<script>alert(1)</script>\n\n[bad](javascript:alert(1))\n\n![pixel](https://example.com/track)';
  const html = renderer.render(source, false).html;
  assert.doesNotMatch(html, /<script|javascript:|<img/i);
  renderer.dispose();
});

test('incremental parsing matches a completed render', () => {
  const renderer = createHtmlRenderer({ controls: false, lineNumbers: false });
  const source = '# Title\n\nA **bold** paragraph.\n\n- One\n- Two\n\n```rust\nlet value = 42;\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |';
  for (let end = 1; end < source.length; end += 7) {
    renderer.render(source.slice(0, end), true, true);
  }
  const final_html = renderer.render(source, false, false).html;
  const fresh = createHtmlRenderer({ controls: false, lineNumbers: false });
  assert.equal(final_html, fresh.render(source, false).html);
  for (const tag of ['h1', 'strong', 'ul', 'pre', 'table']) {
    assert.match(final_html, new RegExp(`<${tag}[ >]`));
  }
  renderer.dispose();
  fresh.dispose();
});

test('only embeds search images selected by the reply', () => {
  const image = 'https://example.com/bridge.jpg';
  const unused = 'https://example.com/unused.jpg';
  const renderer = createHtmlRenderer(imageUrlOptions([image, unused]));
  try {
    const link = renderer.render(`[Photo](${image})`, false).html;
    assert.ok(link.includes(`href="${image}"`));
    assert.doesNotMatch(link, /<img/);

    const reply = `![Bridge](${image}) ![Other](https://example.com/tracker) [Bad](javascript:alert)`;
    const html = renderer.render(reply, false).html;
    assert.ok(html.includes(`src="${image}"`));
    assert.doesNotMatch(html, /unused.jpg|tracker|javascript:/);
  } finally {
    renderer.dispose();
  }
});

test('single newlines stay line breaks outside code, without changing block structure', () => {
  const renderer = createHtmlRenderer({ controls: false, lineNumbers: false });
  try {
    const lines = renderer.render(keepLineBreaks('1\n2\n3'), false).html;
    assert.equal((lines.match(/<br/g) ?? []).length, 2);

    const fixture = readFileSync(new URL('./fixtures/markdown.md', import.meta.url), 'utf8');
    const plain = renderer.render(fixture, false).html;
    const kept = renderer.render(keepLineBreaks(fixture), false).html;
    for (const tag of ['h1', 'ul', 'ol', 'blockquote', 'pre', 'table', 'th', 'td', 'li']) {
      const count = (html) => (html.match(new RegExp(`<${tag}[ >]`, 'g')) ?? []).length;
      assert.equal(count(kept), count(plain), tag);
    }

    const code = '```\nfirst\nsecond\n```';
    assert.equal(keepLineBreaks(code), code);
    assert.equal(keepLineBreaks('a  \nb'), 'a  \nb');
    assert.equal(keepLineBreaks('a\n\nb'), 'a\n\nb');
  } finally {
    renderer.dispose();
  }
});

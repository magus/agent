import { imageUrlOptions } from './imageUrlOptions.js';
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

test('preserves code text and normalizes prose line endings', () => {
  const renderer = createHtmlRenderer({ controls: false, lineNumbers: false });
  const render = (source) => renderer.render(source, false).html;
  try {
    const block_list = [
      '    alpha\n    beta',
      '> ```\n> alpha\n> beta\n> ```',
    ];
    for (const source of block_list) {
      assert.match(render(source), />alpha\nbeta\n<\/span>/);
    }
    assert.match(render('A multiline `alpha\nbeta` span'), />alpha beta<\/code>/);
    assert.match(render('one\ntwo'), />one\ntwo<\/p>/);
    assert.equal(render('one\r\ntwo'), render('one\ntwo'));
  } finally {
    renderer.dispose();
  }
});

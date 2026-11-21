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
  const renderer = createHtmlRenderer({ disallowedElements: ['img'], controls: false });
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

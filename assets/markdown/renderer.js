import { createDomRenderer } from '@pierre/words/html';
import { createHighlightsPlugin } from '@pierre/words/highlights';
import { pierreDark, pierreLight } from '@pierre/highlights/themes';

const root = document.getElementById('message');
const code_by_mode = {
  light: createHighlightsPlugin({ theme: pierreLight }),
  dark: createHighlightsPlugin({ theme: pierreDark }),
};
const renderer = createDomRenderer(root, {
  controls: false,
  lineNumbers: false,
  disallowedElements: ['img'],
});
renderer.connect();
// WebKit otherwise consumes vertical drags over code blocks inside the native scroll view.
document.addEventListener('touchstart', () => {}, { passive: true });
let current_dark;

/** @typedef {{source: string, active: boolean, font_size: number, dark: boolean}} MessageUpdate */

/** @param {MessageUpdate} update */
window.render_message = (update) => {
  document.documentElement.style.fontSize = `${update.font_size}px`;
  document.documentElement.style.colorScheme = update.dark ? 'dark' : 'light';
  if (current_dark !== update.dark) {
    renderer.configure({
      controls: false,
      lineNumbers: false,
      disallowedElements: ['img'],
      plugins: { code: update.dark ? code_by_mode.dark : code_by_mode.light },
    });
    current_dark = update.dark;
  }
  renderer.update(update.source, update.active, update.active, false);
};

let previous_height = 0;
new ResizeObserver(() => {
  const height = Math.ceil(root.getBoundingClientRect().height);
  if (height !== previous_height) {
    previous_height = height;
    window.webkit.messageHandlers.height.postMessage(height);
  }
}).observe(root);

window.addEventListener('pagehide', () => {
  renderer.disconnect();
});

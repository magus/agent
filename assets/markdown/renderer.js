import { imageUrlOptions } from './imageUrlOptions.js';
import { keepLineBreaks } from './lineBreaks.js';
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
let current_images;

/** @typedef {{source: string, active: boolean, font_size: number, dark: boolean, image_urls?: string[]}} MessageUpdate */

/** @param {MessageUpdate} update */
window.render_message = (update) => {
  document.documentElement.style.fontSize = `${update.font_size}px`;
  document.documentElement.style.colorScheme = update.dark ? 'dark' : 'light';
  const images = JSON.stringify(update.image_urls ?? []);
  if (current_dark !== update.dark || current_images !== images) {
    if (current_images !== images) renderer.update('', false, false, false);
    renderer.configure({
      controls: false,
      lineNumbers: false,
      ...imageUrlOptions(update.image_urls ?? []),
      plugins: { code: update.dark ? code_by_mode.dark : code_by_mode.light },
    });
    current_dark = update.dark;
    current_images = images;
  }
  renderer.update(keepLineBreaks(update.source), update.active, update.active, false);
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

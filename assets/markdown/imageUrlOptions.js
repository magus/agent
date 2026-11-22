import { defaultUrlTransform } from '@pierre/words/html';

export function imageUrlOptions(image_urls) {
  const urls = new Set(image_urls.filter(url => url.startsWith('https://')));
  if (!urls.size) return { disallowedElements: ['img'] };

  return {
    urlTransform(url, key) {
      if (key === 'src' && !urls.has(url)) return null;
      return defaultUrlTransform(url);
    },
  };
}

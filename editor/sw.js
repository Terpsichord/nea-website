var cacheName = 'my-ide-pwa';
var filesToCache = [
  './',
  './index.html',
  './editor.js',
  './editor.wasm',
];

/* Start the service worker and cache all of the app's content */
self.addEventListener('install', (e) => {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    })
  );
});

/* Serve cached content when offline */
self.addEventListener('fetch', (e) => {
  let response = fetch(e.request);

  // only use cached files if running as a standalone installed app
  if (window.matchMedia('(display-mode: standalone)').matches) {
    response = response.catch((_error) => {
      return caches.match(e.request)
    })
  }

  e.respondWith(response);
});
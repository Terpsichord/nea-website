var cacheName = 'my-ide-pwa';
var filesToCache = [
  './',
  './index.html',
  './my-ide.js',
  './my-ide.wasm',
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
  e.respondWith(
    fetch(e.request).catch((_error) => {
      return caches.match(e.request)
    })
  );
});
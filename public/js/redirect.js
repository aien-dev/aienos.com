/* Redirect stubs (seo::REDIRECTS): forward to data-to, keeping any ?query
   and #hash. Only a root-relative target is followed. */
(() => {
  const to = document.currentScript && document.currentScript.dataset.to;
  if (typeof to === 'string' && /^\/(?!\/)/.test(to)) location.replace(to + location.search + location.hash);
})();

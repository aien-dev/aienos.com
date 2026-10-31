/* The program runner (/aien/run.html, components::csp). The Aien page frames
   this document sandboxed to allow-scripts and posts one program; it is
   written here, where the policy allows inline scripts. Nothing is written
   unless this document has an opaque origin (a sandboxed frame) and the
   message comes from the framing page. document.open() drops this listener,
   so a document is written once. */
(() => {
  if (self.origin !== 'null' || parent === self) return;
  document.documentElement.style.visibility = 'hidden';
  addEventListener('message', event => {
    if (event.source !== parent || typeof event.data !== 'string') return;
    document.open();
    document.write(event.data);
    document.close();
  });
})();

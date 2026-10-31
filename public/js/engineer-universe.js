// Universe map: planets are plain links to #w-<id>; this only adds keyboard
// travel, Escape to close, and focus handling. Everything works without it.
(function () {
  var uni = document.querySelector('.uni');
  if (!uni) return;
  var worlds = Array.prototype.slice.call(uni.querySelectorAll('.world'));
  function current() {
    var id = location.hash.slice(1);
    return worlds.filter(function (w) { return w.id === id; })[0] || null;
  }
  function syncState() {
    var w = current();
    uni.classList.toggle('has-world', !!w);
    uni.querySelectorAll('.planet').forEach(function (p) {
      p.classList.toggle('is-active', !!w && p.getAttribute('href') === '#' + w.id);
    });
    if (w) { var h = w.querySelector('h2'); if (h) { h.setAttribute('tabindex', '-1'); h.focus({ preventScroll: true }); } }
  }
  window.addEventListener('hashchange', syncState);
  syncState();
  document.addEventListener('keydown', function (e) {
    var w = current();
    if (e.key === 'Escape' && w) { history.pushState(null, '', location.pathname + '#map'); syncState(); return; }
    if (!w || e.target.closest('input,textarea')) return;
    var sel = e.key === 'ArrowRight' ? '.world-next' : e.key === 'ArrowLeft' ? '.world-prev' : null;
    var a = sel && w.querySelector(sel);
    if (a) { e.preventDefault(); location.hash = a.getAttribute('href'); }
  });
})();

/*
 * The Turing look, motion half: the same two effects /turing/ runs.
 * 1. A blue-to-green reading bar that fills as the page scrolls.
 * 2. Sections and cards glide up and fade in once as they come into view,
 *    with cards in a row arriving one after another.
 * Without this script (or with reduced motion asked for) every element is
 * simply shown: the hiding class is only added here.
 */
(function () {
  var root = document.documentElement;
  var calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function start() {
    var bar = document.createElement("div");
    bar.className = "tl-progress";
    bar.setAttribute("aria-hidden", "true");
    document.body.appendChild(bar);
    function onScroll() {
      var room = root.scrollHeight - root.clientHeight;
      bar.style.width = (room > 0 ? (root.scrollTop / room) * 100 : 0).toFixed(2) + "%";
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    onScroll();

    if (calm || !("IntersectionObserver" in window)) return;

    // Blocks fade in whole; cards inside a block arrive one after another.
    var cards = [
      ".hub-card", ".narr-card", ".rs-card", ".blog-card", ".home-q", ".entry",
      ".card", ".research-wrap .artifact", ".receipt", ".pdf-callout",
      ".hub-callout", ".theory-fig", ".loop-stage"
    ].join(",");
    var blocks = [
      ".section-head", ".hub-section", ".narr-section", ".rs-section", ".loop",
      ".home-questions", ".research-wrap .section", ".aien-footer-row"
    ].join(",");
    var fold = window.innerHeight * 0.9;
    var seen = [];
    function add(el) {
      // Leave the first screen alone: it is already in view on arrival.
      if (el.getBoundingClientRect().top < fold) return;
      for (var i = 0; i < seen.length; i++) if (seen[i].contains(el) || el.contains(seen[i])) return;
      seen.push(el);
    }
    document.querySelectorAll(cards).forEach(add);
    document.querySelectorAll(blocks).forEach(function (el) {
      if (!el.querySelector(cards)) add(el);
    });
    if (!seen.length) return;

    var io = new IntersectionObserver(function (entries) {
      var order = 0;
      entries.forEach(function (e) {
        if (!e.isIntersecting) return;
        e.target.style.transitionDelay = Math.min(order++ * 80, 320) + "ms";
        e.target.classList.add("tl-in");
        e.target.addEventListener("transitionend", function clear() {
          e.target.style.transitionDelay = "";
          e.target.removeEventListener("transitionend", clear);
        });
        io.unobserve(e.target);
      });
    }, { threshold: 0.12, rootMargin: "0px 0px -6% 0px" });

    seen.forEach(function (el) {
      el.classList.add("tl-reveal");
      io.observe(el);
    });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
  else start();
})();

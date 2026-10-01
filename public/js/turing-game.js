// The Turing, as a game you can play in thirty seconds.
//
// A hidden rule generates a stream of bits. You call each next bit before
// it lands. A correct call earns 1 T, a miss costs 1 T, and a coin flip
// averages zero: exactly the idea behind the measure. Memorizing is
// useless here because every bit is new; only finding the rule pays.
(function () {
  "use strict";

  var ROUNDS = 12;

  // Each generator is a small hidden rule over the bit history.
  // firstBits are shown for free so the pattern can be spotted.
  var RULES = [
    {
      name: "alternation",
      firstBits: [0, 1, 0, 1, 0, 1],
      next: function (h) { return h[h.length - 1] ^ 1; }
    },
    {
      name: "growing runs",
      firstBits: [0, 1, 1, 0, 0, 0],
      next: function (h) {
        // Run lengths grow by one: 0, 11, 000, 1111, 00000, ...
        var last = h[h.length - 1];
        var s = h.length - 1;
        while (s > 0 && h[s - 1] === last) s--;
        var runsBefore = 0, i = 0;
        while (i < s) {
          var j = i;
          while (j < s && h[j] === h[i]) j++;
          runsBefore++;
          i = j;
        }
        var trailing = h.length - s;
        return trailing < runsBefore + 1 ? last : last ^ 1;
      }
    },
    {
      name: "xor of the last two",
      firstBits: [0, 1, 1, 0, 1, 1],
      next: function (h) { return h[h.length - 1] ^ h[h.length - 2]; }
    },
    {
      name: "pairs",
      firstBits: [1, 1, 0, 0, 1, 1],
      next: function (h) {
        var last = h[h.length - 1];
        var prev = h[h.length - 2];
        return last === prev ? last ^ 1 : last;
      }
    }
  ];

  var streamEl, scoreEl, roundEl, msgEl, btn0, btn1, btnReset;
  var history, rule, score, round, finished;

  function render() {
    streamEl.textContent = history.join(" ") + "  ?";
    scoreEl.textContent = (score > 0 ? "+" : "") + score + " T";
    roundEl.textContent = "Round " + round + " / " + ROUNDS;
  }

  function start() {
    rule = RULES[Math.floor(Math.random() * RULES.length)];
    history = rule.firstBits.slice();
    score = 0;
    round = 0;
    finished = false;
    btn0.disabled = false;
    btn1.disabled = false;
    btn0.style.opacity = "1";
    btn1.style.opacity = "1";
    msgEl.textContent =
      "Six bits are on the table for free. The rest are yours to predict. " +
      "Right call: +1 T. Miss: -1 T. A coin flip averages zero.";
    render();
  }

  function verdict() {
    if (score >= 8) {
      return "That is understanding. A short rule in your head just compressed a stream you had never seen, and it paid out in Turings. The full measure, with sealed data and recomputable receipts, is in the paper.";
    }
    if (score >= 3) {
      return "You are finding the pattern. That climb, from guessing to predicting, is exactly what the Turing measures. Memorizers cannot do it on new data.";
    }
    if (score >= -2) {
      return "About coin-flip territory, which is the point: guessing earns nothing under this unit. The rule was there. Try a fresh stream and hunt for it.";
    }
    return "Below chance this time. The stream beat you, and the scoreboard said so honestly. That honesty is the whole idea: the Turing only pays for real prediction.";
  }

  function guess(bit) {
    if (finished) return;
    var actual = rule.next(history);
    history.push(actual);
    round++;
    if (bit === actual) {
      score += 1;
      msgEl.textContent = "Called it. +1 T. The next bit is already hidden.";
    } else {
      score -= 1;
      msgEl.textContent = "It landed " + actual + ". -1 T. Adjust the rule in your head and call the next one.";
    }
    if (round >= ROUNDS) {
      finished = true;
      btn0.disabled = true;
      btn1.disabled = true;
      btn0.style.opacity = "0.5";
      btn1.style.opacity = "0.5";
      streamEl.textContent = history.join(" ");
      scoreEl.textContent = (score > 0 ? "+" : "") + score + " T";
      roundEl.textContent = "Stream complete";
      msgEl.textContent = verdict();
      return;
    }
    render();
  }

  document.addEventListener("DOMContentLoaded", function () {
    streamEl = document.getElementById("tg-stream");
    if (!streamEl) return;
    scoreEl = document.getElementById("tg-score");
    roundEl = document.getElementById("tg-round");
    msgEl = document.getElementById("tg-msg");
    btn0 = document.getElementById("tg-zero");
    btn1 = document.getElementById("tg-one");
    btnReset = document.getElementById("tg-reset");
    btn0.addEventListener("click", function () { guess(0); });
    btn1.addEventListener("click", function () { guess(1); });
    btnReset.addEventListener("click", start);
    start();
  });
})();

/* brain.js: Allen's local conversation, ported line by line from
 * native/kernel/svc/allen_reply.c and allen_notes.c (aienos-mv, PR #415).
 * No model, no network. Pure functions: loads in a browser or in node.
 *
 * Copied from the Allen phone app (github.com/aien-dev, AGPL-3.0-or-later). This copy adds pageFacts()
 * for the web page ("this browser", nothing kept between visits). The phone app tests the rest against
 * the C code byte for byte.
 *
 * Differences from the original, all on purpose and all listed in
 * tests/diff_test.js (PHONE_DIFFERENCES):
 *   - "this computer" is "this phone".
 *   - The machine answer names cores and screen, not disks.
 *   - "How are you" does not mention a keyboard.
 *   - A clock question gets the real time when the app supplies a clock.
 *   - Wording for "my storage is not ready" says "close this app".
 */
(function (root) {
  "use strict";

  var KEY_MAX = 48, VAL_MAX = 64, FACTS_MAX = 32, NAME_MAX = 24;
  var NOTE_MAX = 128, NOTES_MAX = 512, NOTE_WORDS = 12, OUT_CAP = 160;

  var GOODBYE = "Goodbye. Until next time.";
  var SAVE_FAILED = "I couldn't save that, so I'll only remember it until you close this app.";
  var NOT_YET = "I don't know that yet. Type help to see what I can talk about.";

  /* ---- text helpers (the C works on bytes; the app hands over printable ASCII) ---- */

  /* Lowercase letters, keep digits, every other run becomes one space, trim. */
  function fold(s, cap) {
    var out = "", space = false;
    for (var i = 0; i < s.length && out.length + 1 < cap; i++) {
      var c = s.charAt(i);
      if (c >= "A" && c <= "Z") c = String.fromCharCode(c.charCodeAt(0) + 32);
      if ((c >= "a" && c <= "z") || (c >= "0" && c <= "9")) {
        if (space && out.length) out += " ";
        if (out.length + 1 < cap) out += c;
        space = false;
      } else {
        space = true;
      }
    }
    return out;
  }

  /* The remainder of f after a leading phrase and a space, or null. */
  function after(f, phrase) {
    if (f.length > phrase.length && f.substr(0, phrase.length) === phrase && f.charAt(phrase.length) === " ")
      return f.substr(phrase.length + 1);
    return null;
  }

  /* Where phrase starts in f as whole words, or -1. */
  function find(f, phrase) {
    for (var s = 0; s < f.length; s++) {
      if (s !== 0 && f.charAt(s - 1) !== " ") continue;
      if (f.substr(s, phrase.length) === phrase) {
        var e = s + phrase.length;
        if (e === f.length || f.charAt(e) === " ") return s;
      }
    }
    return -1;
  }

  function any(f, list) {
    for (var i = 0; i < list.length; i++) if (find(f, list[i]) >= 0) return true;
    return false;
  }

  /* The answer being composed. Stops at OUT_CAP - 1 characters like the C buffer. */
  function Say() { this.b = ""; }
  Say.prototype.put = function (t) {
    for (var i = 0; i < t.length && this.b.length + 1 < OUT_CAP; i++) this.b += t.charAt(i);
  };
  Say.prototype.putu = function (v) { this.put(String(v)); };
  Say.prototype.putCount = function (v, one, many) { this.putu(v); this.put(" "); this.put(v === 1 ? one : many); };
  Say.prototype.putAwake = function (t) {
    if (t >= 3600) {
      this.putCount(Math.floor(t / 3600), "hour", "hours");
      if (Math.floor((t % 3600) / 60)) {
        this.put(" and ");
        this.putCount(Math.floor((t % 3600) / 60), "minute", "minutes");
      }
    } else if (t >= 60) {
      this.putCount(Math.floor(t / 60), "minute", "minutes");
      if (t % 60) {
        this.put(" and ");
        this.putCount(t % 60, "second", "seconds");
      }
    } else {
      this.putCount(t, "second", "seconds");
    }
  };

  function putName(s, m) {
    if (m.mem.name) { s.put(", "); s.put(m.mem.name); }
  }

  /* ---- numbers and sums ---- */

  var NUMBER_WORDS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen", "twenty"];

  /* A number at the start of p (digits up to 9, or a word zero to twenty)
   * followed by a space or the end. {v, rest} or null. */
  function number(p) {
    var v = -1, s = 0, i;
    var c = p.charAt(0);
    if (c >= "0" && c <= "9") {
      var digits = 0;
      v = 0;
      while (s < p.length && p.charAt(s) >= "0" && p.charAt(s) <= "9") {
        v = v * 10 + (p.charCodeAt(s) - 48);
        s++; digits++;
      }
      if (digits > 9) return null;
    } else {
      for (i = 0; i < 21 && v < 0; i++) {
        var w = NUMBER_WORDS[i];
        if (p.substr(0, w.length) === w && (p.length === w.length || p.charAt(w.length) === " ")) {
          v = i;
          s = w.length;
        }
      }
    }
    if (v < 0) return null;
    if (s < p.length && p.charAt(s) !== " ") return null;
    return { v: v, rest: s < p.length ? p.substr(s + 1) : "" };
  }

  var LEAD_SUM = ["what is", "whats", "what s", "calculate", "how much is"];
  var OPS = [["plus", "+"], ["add", "+"], ["minus", "-"], ["take away", "-"], ["times", "*"], ["x", "*"],
    ["multiplied by", "*"], ["divided by", "/"], ["over", "/"]];

  function arithmetic(s, f) {
    var p = f, r, i;
    for (i = 0; i < LEAD_SUM.length; i++)
      if ((r = after(f, LEAD_SUM[i])) !== null) { p = r; break; }
    var a = number(p);
    if (!a) return false;
    p = a.rest;
    var op = "";
    for (i = 0; i < OPS.length && !op; i++) {
      if ((r = after(p, OPS[i][0])) !== null) { op = OPS[i][1]; p = r; }
    }
    if (!op) return false;
    var b = number(p);
    if (!b || b.rest) return false;
    if (op === "/" && b.v === 0) {
      s.put("Nobody can divide by zero, not even me.");
      return true;
    }
    var A = BigInt(a.v), B = BigInt(b.v);
    s.putu(a.v);
    s.put(op === "+" ? " plus " : op === "-" ? " minus " : op === "*" ? " times " : " divided by ");
    s.putu(b.v);
    s.put(" is ");
    var v = op === "+" ? A + B : op === "-" ? A - B : op === "*" ? A * B : A / B;
    if (v < 0n) { s.put("minus "); v = -v; }
    s.put(v.toString());
    if (op === "/" && A % B !== 0n) { s.put(" remainder "); s.put((A % B).toString()); }
    s.put(".");
    return true;
  }

  /* ---- names and facts ---- */

  var STATE_WORDS = ["good", "fine", "great", "ok", "okay", "well", "tired", "sad", "happy", "bored", "here",
    "back", "sorry", "not", "so", "very", "doing", "just", "really", "excited", "shook", "alright", "awesome"];

  function learnName(m, rest) {
    var n = 0;
    while (n < rest.length && rest.charAt(n) !== " " && n < 23) n++;
    var w = rest.substr(0, n);
    if (!n || rest.charAt(n) === " " || STATE_WORDS.indexOf(w) >= 0 || w === "allen") return false;
    m.mem.name = w.charAt(0).toUpperCase() + w.substr(1);
    m.changed = true;
    return true;
  }

  /* t as Allen says it back: "my" becomes "your", word by word. */
  var TURN_FROM = ["my", "your", "i", "me", "mine", "yours", "myself", "am"];
  var TURN_TO = ["your", "my", "you", "you", "yours", "mine", "yourself", "are"];
  function putTurned(s, t, cap) {
    var out = "";
    while (t.length) {
      var n = 0;
      while (n < t.length && t.charAt(n) !== " " && n < VAL_MAX - 1) n++;
      var w = t.substr(0, n);
      t = t.substr(n);
      var say = w, k = TURN_FROM.indexOf(w);
      if (k >= 0) say = TURN_TO[k];
      if (cap && say.charAt(0) >= "a" && say.charAt(0) <= "z") say = say.charAt(0).toUpperCase() + say.substr(1);
      cap = false;
      out += say;
      if (t.charAt(0) === " ") { out += " "; t = t.substr(1); }
    }
    s.put(out);
  }

  function factIndex(m, key) {
    for (var i = 0; i < m.mem.facts.length && i < FACTS_MAX; i++) if (m.mem.facts[i].key === key) return i;
    return -1;
  }

  function putFact(s, f, cap) {
    putTurned(s, f.key, cap);
    s.put(f.are ? " are " : " is ");
    putTurned(s, f.val, false);
  }

  function savedNote(m) {
    return m.lasting ? " I'll remember that." : " I'll remember that " + m.facts.untilOff + ".";
  }

  function teach(m, s, f) {
    var rest = after(f, "remember that");
    if (rest === null) rest = after(f, "remember");
    if (rest === null) return false;
    var verb = -1;
    for (var p = 0; p < rest.length && verb < 0; p++)
      if (rest.charAt(p) === " " && (after(rest.substr(p + 1), "is") !== null || after(rest.substr(p + 1), "are") !== null)) verb = p;
    if (verb < 0) {
      s.put("Teach me like this: remember my favorite color is blue.");
      return true;
    }
    var are = rest.charAt(verb + 1) === "a";
    var klen = verb;
    var val = rest.substr(verb + (are ? 5 : 4));
    var vlen = val.length;
    if (klen === 7 && after(rest, "my name") !== null && !are) {
      if (learnName(m, val)) {
        s.put("Nice to meet you, ");
        s.put(m.mem.name);
        s.put(".");
        s.put(savedNote(m));
      } else {
        s.put("Tell me your name as one word, like: my name is Drake.");
      }
      return true;
    }
    if (klen >= KEY_MAX || vlen >= VAL_MAX) {
      s.put("That's longer than I can hold yet. Try something shorter.");
      return true;
    }
    var key = rest.substr(0, klen);
    var i = factIndex(m, key);
    if (i < 0) {
      if (m.mem.facts.length >= FACTS_MAX) {
        s.put("My memory is full. Type forget and something I know to make room.");
        return true;
      }
      m.mem.facts.push({ key: key, val: "", are: 0 });
      i = m.mem.facts.length - 1;
    }
    m.mem.facts[i].val = val;
    m.mem.facts[i].are = are ? 1 : 0;
    m.changed = true;
    s.put("Got it: ");
    putFact(s, m.mem.facts[i], false);
    s.put(".");
    s.put(savedNote(m));
    return true;
  }

  var RECALL_LEAD = ["what is", "what are", "whats", "what s", "who is", "who are", "where is", "where are",
    "when is", "do you remember", "tell me"];

  function recall(m, s, f, ctx) {
    for (var i = 0; i < RECALL_LEAD.length; i++) {
      var key = after(f, RECALL_LEAD[i]);
      if (key === null) continue;
      var k = factIndex(m, key);
      if (k >= 0) {
        putFact(s, m.mem.facts[k], true);
        s.put(".");
        return true;
      }
      if (after(key, "my") !== null) ctx.askedMy = key;
    }
    return false;
  }

  function forget(m, s, f) {
    var rest = after(f, "forget about");
    if (rest === null) rest = after(f, "forget that");
    if (rest === null) rest = after(f, "forget");
    if (rest === null) return false;
    if (rest === "everything" || rest === "all" || rest === "it all" || rest === "all of it" || rest === "me") {
      m.mem.facts = [];
      m.mem.name = "";
      m.changed = true;
      s.put("Done. I've forgotten everything you taught me, and your name.");
      return true;
    }
    if (rest === "my name") {
      m.mem.name = "";
      m.changed = true;
      s.put("Done. I've forgotten your name.");
      return true;
    }
    var i = factIndex(m, rest);
    if (i < 0) {
      s.put("I don't remember anything about ");
      putTurned(s, rest, false);
      s.put(".");
      return true;
    }
    m.mem.facts.splice(i, 1);
    m.changed = true;
    s.put("Done. I've forgotten ");
    putTurned(s, rest, false);
    s.put(".");
    return true;
  }

  function listMemory(m, s) {
    var n = m.mem.facts.length + (m.mem.name ? 1 : 0);
    if (!n) {
      s.put("Nothing yet. Teach me like this: remember my favorite color is blue.");
      return;
    }
    s.put("I remember ");
    s.putCount(n, "thing", "things");
    s.put(":");
    var told = 0;
    if (m.mem.name) {
      s.put(" your name is ");
      s.put(m.mem.name);
      told++;
    }
    for (var i = 0; i < m.mem.facts.length; i++) {
      var t = new Say();
      t.put(told ? "; " : " ");
      putFact(t, m.mem.facts[i], false);
      /* the C one-piece buffer is KEY_MAX + VAL_MAX + 16 bytes */
      if (t.b.length > KEY_MAX + VAL_MAX + 15) t.b = t.b.substr(0, KEY_MAX + VAL_MAX + 15);
      if (s.b.length + t.b.length + 12 >= OUT_CAP) {
        s.put("; and more");
        break;
      }
      s.put(t.b);
      told++;
    }
    s.put(".");
  }

  /* ---- notes (allen_notes.c) ---- */

  var SMALL_WORDS = ["the", "an", "and", "or", "but", "is", "are", "was", "were", "be", "been", "am", "me", "my",
    "mine", "you", "your", "yours", "we", "our", "it", "its", "this", "that", "these", "those", "of", "in", "on",
    "at", "to", "for", "from", "with", "about", "as", "by", "what", "whats", "where", "when", "who", "which",
    "why", "how", "do", "does", "did", "have", "has", "had", "can", "could", "would", "should", "will", "there",
    "here", "then", "than", "so", "not", "no", "yes", "just", "also", "please", "show", "tell", "give", "find",
    "search", "look", "up", "get", "got", "any", "anything", "something", "thing", "things", "note", "notes",
    "saved", "save", "remember", "know", "all", "some", "again", "more", "ok", "okay", "hey", "hi", "hello",
    "allen", "want", "need", "let", "lets", "remind", "pull", "down", "write", "jot", "keep"];

  function hash(w) {
    var h = 2166136261;
    for (var i = 0; i < w.length; i++) h = Math.imul((h ^ w.charCodeAt(i)) >>> 0, 16777619) >>> 0;
    return h ? h : 1;
  }

  function isAlnum(c) { return (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || (c >= "0" && c <= "9"); }

  /* Up to max key words of text as hashes, in order, no duplicates. */
  function keyWords(text, max) {
    var words = [], s = 0;
    while (s < text.length && words.length < max) {
      var w = "", digits = true;
      while (s < text.length && !isAlnum(text.charAt(s))) s++;
      while (s < text.length && isAlnum(text.charAt(s))) {
        var c = text.charAt(s++).toLowerCase();
        if (c < "0" || c > "9") digits = false;
        if (w.length < 32) w += c;
      }
      if (!w.length || w.length === 32) continue;
      if (!digits && (w.length < 3 || SMALL_WORDS.indexOf(w) >= 0)) continue;
      if (!digits && w.length > 3 && w.charAt(w.length - 1) === "s" && w.charAt(w.length - 2) !== "s")
        w = w.substr(0, w.length - 1);
      var h = hash(w);
      if (words.indexOf(h) < 0) words.push(h);
    }
    return words;
  }

  /* Text as the C accepts it: printable ASCII, trimmed, 1 to NOTE_MAX - 1 long. Or null. */
  function noteText(text) {
    var t = text.replace(/^ +/, "");
    for (var i = 0; i < t.length; i++) {
      var c = t.charCodeAt(i);
      if (c < 0x20 || c > 0x7e) return null;
    }
    t = t.replace(/ +$/, "");
    if (!t.length || t.length >= NOTE_MAX) return null;
    return t;
  }

  function makeNote(text, start) {
    var t = noteText(text);
    if (t === null) return null;
    return { text: t, start: start, gone: 0, word: keyWords(t, NOTE_WORDS) };
  }

  function noteLinks(a, b) {
    var k = 0;
    for (var i = 0; i < a.word.length; i++) for (var j = 0; j < b.word.length; j++) k += a.word[i] === b.word[j] ? 1 : 0;
    return k;
  }

  function noteDegree(notes, i) {
    var d = 0;
    for (var j = 0; j < notes.length; j++) d += j !== i && !notes[j].gone && noteLinks(notes[i], notes[j]) > 0 ? 1 : 0;
    return d;
  }

  function notesLive(notes) {
    var k = 0;
    for (var i = 0; i < notes.length; i++) k += notes[i].gone ? 0 : 1;
    return k;
  }

  /* {index, also} of the best live note for the question, or index -1. */
  function notesBest(notes, question, strict) {
    var q = keyWords(question, NOTE_WORDS), nq = q.length, bestScore = 0, count = 0, best = -1;
    if (!nq) return { index: -1, also: 0 };
    for (var i = 0; i < notes.length; i++) {
      var n = notes[i];
      if (n.gone) continue;
      var score = 0;
      for (var a = 0; a < nq; a++)
        for (var b = 0; b < n.word.length; b++)
          if (q[a] === n.word[b]) { score++; break; }
      if (!score || (strict ? score < nq : score * 2 < nq)) continue;
      count++;
      if (score >= bestScore) { bestScore = score; best = i; }
    }
    return { index: best, also: count ? count - 1 : 0 };
  }

  /* line past its first `words` words and the spaces, colons, commas, dashes after them */
  function rawAfter(line, words) {
    var p = 0;
    for (var k = 0; k < words; k++) {
      while (p < line.length && !isAlnum(line.charAt(p))) p++;
      while (p < line.length && isAlnum(line.charAt(p))) p++;
    }
    while (p < line.length && " :,-".indexOf(line.charAt(p)) >= 0) p++;
    return line.substr(p);
  }

  function wordsIn(phrase) { return phrase.split(" ").length; }

  /* {lead, rest} for the first leading phrase of f in list, or null. */
  function leadOf(f, list) {
    for (var i = 0; i < list.length; i++) {
      if (f === list[i]) return { lead: list[i], rest: "" };
      var r = after(f, list[i]);
      if (r !== null) return { lead: list[i], rest: r };
    }
    return null;
  }

  function handNote(m, s, i, also) {
    var n = m.notes[i];
    s.put("From your notes: ");
    s.put(n.text);
    var last = n.text.charAt(n.text.length ? n.text.length - 1 : 0);
    if (last !== "." && last !== "!" && last !== "?") s.put(".");
    if (also && s.b.length + 24 < OUT_CAP) {
      s.put(" I have ");
      s.putu(also);
      s.put(" more on that.");
    }
    m.lit = i + 1;
    m.mapChanged = true;
  }

  function listNotes(m, s) {
    var live = notesLive(m.notes);
    if (!live) {
      s.put("You haven't saved any notes yet. Save one like this: save the gate code is 4471.");
      return;
    }
    s.put("You have ");
    s.putCount(live, "note", "notes");
    s.put(". The newest: ");
    for (var i = m.notes.length; i-- > 0;)
      if (!m.notes[i].gone) { s.put(m.notes[i].text); break; }
    s.put(".");
  }

  var SAVE_LEAD = ["make a note that", "make a note", "write down that", "write down", "jot down", "note that",
    "save that", "save this", "note", "save"];

  function noteSave(m, s, f, line) {
    var l = leadOf(f, SAVE_LEAD);
    if (!l) return false;
    var text = rawAfter(line, wordsIn(l.lead));
    if (!l.rest || !text) {
      s.put("Tell me what to save, like: save the gate code is 4471.");
      return true;
    }
    var want = fold(text, NOTE_MAX), i;
    for (i = 0; i < m.notes.length; i++) {
      if (m.notes[i].gone) continue;
      if (fold(m.notes[i].text, NOTE_MAX) === want) {
        s.put("I already have that saved.");
        m.lit = i + 1;
        m.mapChanged = true;
        return true;
      }
    }
    if (m.notes.length >= NOTES_MAX) {
      s.put("My notebook is full. Delete one to make room, like: delete the note about the gate.");
      return true;
    }
    var note = makeNote(text, m.mem.starts);
    if (note === null) {
      s.put("That's longer than I can hold yet. Try something shorter.");
      return true;
    }
    m.notes.push(note);
    i = m.notes.length - 1;
    m.changed = true;
    m.noteDirty = i + 1;
    m.lit = i + 1;
    m.mapChanged = true;
    var links = noteDegree(m.notes, i);
    s.put("Saved.");
    if (notesLive(m.notes) === 1) s.put(" It's your first note.");
    else if (links) { s.put(" It links to "); s.putCount(links, "other note.", "other notes."); }
    else s.put(" It doesn't link to any other note yet.");
    if (!m.lasting) s.put(" Only " + m.facts.untilOff + ": " + m.facts.notReady + ".");
    return true;
  }

  var FORGET_NOTE_LEAD = ["forget the note about", "forget my note about", "forget the note", "delete the note about",
    "delete my note about", "delete the note", "delete note", "remove the note about", "remove the note",
    "erase the note about"];

  function noteForget(m, s, f) {
    var l = leadOf(f, FORGET_NOTE_LEAD);
    if (!l) return false;
    var best = notesBest(m.notes, l.rest, false);
    var i = best.index;
    if (i < 0) {
      s.put("I have no note about ");
      putTurned(s, l.rest ? l.rest : "that", false);
      s.put(".");
      return true;
    }
    m.notes[i].gone = 1;
    m.noteDirty = i + 1;
    m.lit = 0;
    m.mapChanged = true;
    m.changed = true;
    s.put("Deleted the note: ");
    s.put(m.notes[i].text);
    s.put(".");
    return true;
  }

  var FIND_LEAD = ["do i have anything about", "do i have anything on", "what do i have about", "what do i have on",
    "anything about", "anything on", "search for", "search", "look up", "find", "show me", "pull up",
    "remind me about", "remind me of", "get me", "give me"];
  var LISTING = ["how many notes", "my notes", "list my notes", "list notes", "what notes do i have", "show my notes"];

  function noteFind(m, s, f) {
    var l = leadOf(f, LISTING);
    if (l && !l.rest) { listNotes(m, s); return true; }
    l = leadOf(f, FIND_LEAD);
    if (!l) return false;
    if (!keyWords(l.rest, 1).length) {
      if (find(l.rest, "note") < 0 && find(l.rest, "notes") < 0) return false;
      listNotes(m, s);
      return true;
    }
    var best = notesBest(m.notes, l.rest, false);
    if (best.index < 0) {
      s.put("I have nothing saved about ");
      putTurned(s, l.rest, false);
      s.put(".");
      return true;
    }
    handNote(m, s, best.index, best.also);
    return true;
  }

  var ASK_WORDS = ["what", "whats", "where", "when", "who", "which", "how", "is", "are", "do", "does", "did", "can",
    "tell", "remind"];

  function noteAnswer(m, s, f) {
    var asking = false;
    for (var i = 0; i < ASK_WORDS.length && !asking; i++) asking = f === ASK_WORDS[i] || after(f, ASK_WORDS[i]) !== null;
    if (!asking) return false;
    var best = notesBest(m.notes, f, true);
    if (best.index < 0) return false;
    handNote(m, s, best.index, best.also);
    return true;
  }

  /* ---- the mind ---- */

  function createMind(facts) {
    return {
      mem: { name: "", facts: [], starts: 0, lines: 0 },
      notes: [],
      facts: facts || { cores: 0, screenW: 0, screenH: 0 },
      lasting: false,
      turns: 0,
      askedHow: false,
      changed: false,
      lit: 0,
      mapChanged: false,
      noteDirty: 0,
      clock: null
    };
  }

  function welcome(m) {
    var s = new Say();
    var notes = notesLive(m.notes);
    if (!m.mem.name && !m.mem.facts.length && !notes) return null;
    s.put("Welcome back");
    putName(s, m);
    s.put(".");
    if (m.mem.facts.length) {
      s.put(" I still remember ");
      s.putCount(m.mem.facts.length, "thing", "things");
      s.put(notes ? " you taught me, and your " : " you taught me.");
    } else if (notes) {
      s.put(" I still have your ");
    }
    if (notes) s.putCount(notes, "note.", "notes.");
    return s.b;
  }

  var BAD = ["bad", "not good", "not great", "sad", "tired", "awful", "terrible", "sick"];
  var FINE = ["good", "fine", "great", "ok", "okay", "well", "alright", "not bad", "awesome", "amazing", "excited", "happy"];
  var HOW = ["how are you", "how r you", "how are u", "how you doing", "how are things", "how s it going",
    "hows it going", "how do you feel"];
  var HELP = ["help", "what can you do", "what do you do", "what can i say", "what can i ask", "commands"];
  var YOUR_NAME = ["what is your name", "whats your name", "what s your name", "your name"];
  var MAKER = ["who made you", "who built you", "who created you", "who wrote you", "who programmed you",
    "who is your creator", "who owns you"];
  var WHAT_ARE = ["what are you", "are you an ai", "are you ai", "are you alive", "are you real", "are you a robot",
    "are you conscious", "are you smart", "are you a computer"];
  var MACHINE = ["computer", "machine", "hardware", "specs", "cores", "cpu", "screen", "disk", "disks", "drive"];
  var STARTS = ["how many times", "how many starts", "restarts", "restarted"];
  var AWAKE = ["how long", "uptime", "awake", "how old are you"];
  var CLOCK = ["what time", "time is it", "what day", "date", "time"];
  var REMEMBER = ["remember", "learn", "teach", "memory", "forget"];
  var JOKE = ["joke", "funny", "make me laugh"];
  var JOKES = [
    "Why did the scarecrow win an award? He was outstanding in his field.",
    "I told my computer I needed a break. It said no problem, it would go to sleep.",
    "What do you call a fish with no eyes? A fsh.",
    "Why don't skeletons fight each other? They don't have the guts."
  ];
  var THANKS = ["thank you", "thanks", "thank u", "thx", "ty", "appreciate it"];
  var PRAISE = ["love you", "you re cool", "you re awesome", "you re great", "you re amazing", "you re smart",
    "good job", "well done", "cool", "awesome", "amazing", "nice", "wow"];
  var UP = ["what s up", "whats up", "sup", "what are you doing"];
  var TEST = ["test", "testing", "can you hear me", "are you there"];
  var YES = ["yes", "yeah", "yep", "ok", "okay", "sure", "alright"];
  var HELLO = ["hello", "hi", "hey", "howdy", "greetings", "yo"];
  var ASK_NAME = ["what is my name", "whats my name", "what s my name", "who am i", "do you know my name",
    "do you remember me", "my name"];
  var WHAT_KNOWN = ["what do you remember", "what do you know", "what have you learned", "what have you learnt",
    "what did i teach you", "what did i tell you"];

  function answer(m, line, awakeS) {
    line = line || "";
    var s = new Say();
    var f = fold(line, 96);
    var askedHow = m.askedHow;
    m.askedHow = false;
    m.turns++;
    m.mem.lines++;
    var rest, feel = null;

    if (f === "goodbye" || f === "goodbye allen" || f === "bye" || f === "bye allen" || f === "good bye") return GOODBYE;
    if (f === "hello allen" || f === "hi allen" || f === "hey allen" || f === "hello world") return "Hello Humanity.";
    if (!f) return NOT_YET;
    if (noteForget(m, s, f) || noteSave(m, s, f, line) || noteFind(m, s, f)) return s.b;
    var ctx = { askedMy: null };
    if (teach(m, s, f) || forget(m, s, f) || recall(m, s, f, ctx)) return s.b;
    if (any(f, WHAT_KNOWN)) { listMemory(m, s); return s.b; }
    if (arithmetic(s, f)) return s.b;

    if ((rest = after(f, "my name is")) !== null || (rest = after(f, "call me")) !== null ||
        (rest = after(f, "i am")) !== null || (rest = after(f, "i m")) !== null || (rest = after(f, "im")) !== null) {
      if (learnName(m, rest)) {
        s.put("Nice to meet you, ");
        s.put(m.mem.name);
        s.put(".");
        s.put(savedNote(m));
        return s.b;
      }
      feel = rest;
    }
    if (any(f, ASK_NAME)) {
      if (!m.mem.name) return "You haven't told me yet. Say: my name is, and then your name.";
      s.put("You're ");
      s.put(m.mem.name);
      s.put(".");
      return s.b;
    }

    if ((askedHow || feel !== null) && any(feel !== null ? feel : f, BAD)) return "Sorry to hear that. I'm here if you want to talk.";
    if ((askedHow || feel !== null) && any(feel !== null ? feel : f, FINE)) return "Glad to hear it.";

    if (any(f, HOW)) {
      s.put("I'm running well");
      putName(s, m);
      s.put(". ");
      if (m.facts.cores) {
        s.putCount(m.facts.cores, "core", "cores");
        s.put(" awake, " + (m.facts.keyboardWord || "keyboard working") + ". ");
      }
      s.put("How are you?");
      m.askedHow = true;
      return s.b;
    }
    if (any(f, HELP))
      return "I learn what you teach me (remember my car is red) and keep notes (save the gate code is 4471), " +
        "then hand them back when you ask. Type goodbye to end.";
    if (any(f, YOUR_NAME)) return "My name is Allen.";
    if (any(f, MAKER)) return "Drake built me, from the ground up.";
    if (any(f, WHAT_ARE))
      return "I'm a small program living inside AIENOS. For now I answer from what I was taught by heart, but I'm " +
        "meant to grow.";
    if (find(f, "who are you") >= 0) return "I am Allen. I live in AIENOS.";
    if (find(f, "where are you") >= 0) return "Right here, on this " + m.facts.thing_word() + ". Everything I know stays on it.";
    if (find(f, "aienos") >= 0)
      return "AIENOS is the operating system I live in, built from scratch. There is no other system underneath me.";
    if (any(f, MACHINE)) {
      s.put("This " + m.facts.thing_word() + " has ");
      m.facts.machine_line(s);
      return s.b;
    }
    if (any(f, STARTS) && m.mem.starts) {
      s.put("This is start number ");
      s.putu(m.mem.starts);
      s.put(" for me, and I've answered ");
      s.putCount(m.mem.lines, "line", "lines");
      s.put(" in all.");
      return s.b;
    }
    if (noteAnswer(m, s, f)) return s.b;
    if (any(f, AWAKE) || any(f, CLOCK)) {
      var wantsClock = any(f, CLOCK) && !any(f, AWAKE);
      if (wantsClock && m.clock) {
        s.put(m.clock(f));
        return s.b;
      }
      if (wantsClock) s.put("I can't read the clock yet. ");
      s.put(find(f, "how old are you") >= 0 ? "I was born when this " + m.facts.thing_word() + " started, " : "I've been awake for ");
      s.putAwake(awakeS || 0);
      s.put(find(f, "how old are you") >= 0 ? " ago." : ".");
      return s.b;
    }
    if (any(f, REMEMBER))
      return m.lasting
        ? "Teach me like this: remember my favorite color is blue. Then ask: what is my favorite color. I keep it on this " + m.facts.thing_word() + "'s own " + m.facts.storageWord + "."
        : "Teach me like this: remember my favorite color is blue. Then ask: what is my favorite color. " + m.facts.notReady.charAt(0).toUpperCase() + m.facts.notReady.substr(1) + ", so only " + m.facts.untilOff + ".";
    if (any(f, JOKE)) return JOKES[m.turns % 4];
    if (any(f, THANKS)) {
      s.put("You're welcome");
      putName(s, m);
      s.put(".");
      return s.b;
    }
    if (any(f, PRAISE)) return "Thank you. That means a lot coming from you.";
    if (find(f, "sorry") >= 0) return "No need to be sorry.";
    if (any(f, UP)) return "Just running, and talking with you.";
    if (find(f, "good morning") >= 0 || find(f, "good afternoon") >= 0 || find(f, "good evening") >= 0) {
      s.put(find(f, "good morning") >= 0 ? "Good morning" : find(f, "good afternoon") >= 0 ? "Good afternoon" : "Good evening");
      putName(s, m);
      s.put(".");
      return s.b;
    }
    if (find(f, "good night") >= 0) return "Good night. Type goodbye when you want " + m.facts.goodNight + ".";
    if (any(f, TEST)) return "I hear you. Everything is working.";
    if (any(f, YES)) return "Okay.";
    if (f === "no" || f === "nope") return "Alright.";
    if (any(f, HELLO)) {
      s.put("Hello");
      putName(s, m);
      s.put(m.mem.name ? ". Good to see you." : ". I am Allen.");
      return s.b;
    }
    if (ctx.askedMy !== null) {
      s.put("You haven't told me ");
      putTurned(s, ctx.askedMy, false);
      s.put(" yet. Teach me: remember ");
      s.put(ctx.askedMy);
      s.put(" is, and then the answer.");
      return s.b;
    }
    return NOT_YET;
  }

  /* ---- what the phone says about itself (the original names computer, cores, screen, disks) ---- */

  function phoneFacts(cores, w, h) {
    return {
      cores: cores || 0,
      screenW: w || 0,
      screenH: h || 0,
      keyboardWord: "ready to talk",
      untilOff: "until you close this app", notReady: "my storage isn't ready", goodNight: "me to sleep", storageWord: "storage",
      thing_word: function () { return "phone"; },
      machine_line: function (s) {
        var parts = [];
        if (this.cores) parts.push(this.cores + (this.cores === 1 ? " core" : " cores"));
        if (this.screenW) parts.push("a " + this.screenW + " by " + this.screenH + " screen");
        s.put(parts.length ? parts.join(" and ") + "." : "a screen I can't measure yet.");
      }
    };
  }

  /* What the web page says about itself: this browser, and nothing kept once the page closes. */
  function pageFacts(cores, w, h) {
    var f = phoneFacts(cores, w, h);
    f.untilOff = "until you close this page";
    f.notReady = "I keep nothing between visits";
    f.thing_word = function () { return "browser"; };
    return f;
  }

  /* The original's own wording, for the differential test against the C. */
  function originalFacts(cores, w, h, sata, nvme) {
    return {
      cores: cores, screenW: w, screenH: h,
      keyboardWord: "keyboard working",
      untilOff: "until this computer turns off", notReady: "my disk isn't ready", goodNight: "this computer to turn off", storageWord: "disk",
      thing_word: function () { return "computer"; },
      machine_line: function (s) {
        if (cores) { s.putCount(cores, "core", "cores"); s.put(", "); }
        if (w) { s.put("a "); s.putu(w); s.put(" by "); s.putu(h); s.put(" screen, "); }
        s.putCount(sata, "hard drive", "hard drives");
        s.put(" and ");
        s.putCount(nvme, "SSD", "SSDs");
        s.put(".");
      }
    };
  }

  /* ---- keeping it: a plain object for JSON ---- */

  function serialize(m) {
    return {
      v: 1,
      name: m.mem.name,
      facts: m.mem.facts.map(function (f) { return [f.key, f.val, f.are]; }),
      notes: m.notes.map(function (n) { return [n.text, n.start, n.gone]; }),
      starts: m.mem.starts,
      lines: m.mem.lines
    };
  }

  function restore(m, o) {
    if (!o || o.v !== 1) return false;
    var name = typeof o.name === "string" ? o.name.substr(0, NAME_MAX - 1) : "";
    var facts = [], notes = [], i;
    if (Array.isArray(o.facts))
      for (i = 0; i < o.facts.length && facts.length < FACTS_MAX; i++) {
        var f = o.facts[i];
        if (Array.isArray(f) && typeof f[0] === "string" && typeof f[1] === "string" && f[0].length < KEY_MAX && f[1].length < VAL_MAX)
          facts.push({ key: f[0], val: f[1], are: f[2] ? 1 : 0 });
      }
    if (Array.isArray(o.notes))
      for (i = 0; i < o.notes.length && notes.length < NOTES_MAX; i++) {
        var n = o.notes[i];
        var made = Array.isArray(n) && typeof n[0] === "string" ? makeNote(n[0], n[1] | 0) : null;
        if (made) { made.gone = n[2] ? 1 : 0; notes.push(made); }
      }
    m.mem.name = name;
    m.mem.facts = facts;
    m.notes = notes;
    m.mem.starts = Math.max(0, o.starts | 0);
    m.mem.lines = Math.max(0, o.lines | 0);
    return true;
  }

  /* What the phone's keyboard may hand over, made printable ASCII like the original expects. */
  function clean(line) {
    return String(line || "")
      .replace(/[‘’ʼ]/g, "'")
      .replace(/[“”]/g, '"')
      .replace(/[‐-―−]/g, "-")
      .replace(/…/g, "...")
      .replace(/[\t\r\n ]/g, " ")
      .replace(/[^\x20-\x7e]/g, "")
      .substr(0, 200);
  }

  var api = {
    createMind: createMind, answer: answer, welcome: welcome, serialize: serialize, restore: restore,
    phoneFacts: phoneFacts, pageFacts: pageFacts, originalFacts: originalFacts, clean: clean,
    notesLive: notesLive, noteDegree: noteDegree, noteLinks: noteLinks, keyWords: keyWords,
    GOODBYE: GOODBYE, SAVE_FAILED: SAVE_FAILED,
    limits: { KEY_MAX: KEY_MAX, VAL_MAX: VAL_MAX, FACTS_MAX: FACTS_MAX, NOTE_MAX: NOTE_MAX, NOTES_MAX: NOTES_MAX }
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.AllenBrain = api;
})(typeof self !== "undefined" ? self : this);

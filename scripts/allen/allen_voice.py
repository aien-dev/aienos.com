"""Aien's pre-recorded voice clips: choose clip ids for a turn, or none (the reply stays text-only).

The catalog (public/aien/voice/catalog.json, built by the AienVoice lane) lists every clip with
id, situation, tags, intent, text, file, duration, sha256, license and source. pick() maps a turn
to a situation, in this order:
  1. an explicit situation from the caller (greeting, goodbye, error.busy, build.success, ...);
  2. a pure first-turn greeting: time-of-day or return-visit greeting, then the intro menu;
  3. the cached answer id (answers.json), e.g. "omega" -> explain.omega;
  4. a short table of visitor intents (thanks, bye, "are you Allen", donations, vibe coding, ...).
No match returns []. Clips whose feature is not live ("requires") are never chosen.
Choice inside a situation is deterministic (hash of the turn), skipping clips already played.
Stdlib only.
"""
import hashlib
import json
import re

ANSWER_SITUATION = {
    "si": "explain.sovereign_intelligence", "omega": "explain.omega", "aienos": "explain.aienos",
    "cudapp-what": "explain.cudaplusplus", "install": "install.how", "drake": "explain.drake",
    "learn": "learn.overview", "allen": "smalltalk.who_are_you",
}

# (situation, pattern over the visitor's message); first match wins.
INTENTS = [
    ("smalltalk.are_you_allen", r"\b(are you|r u|is this) allen\b|\bwhat happened to allen\b"),
    ("smalltalk.are_you_human", r"\bare you (a )?(human|real|person|bot|robot|an? ai)\b"),
    ("smalltalk.who_made_you", r"\bwho (made|built|created|makes) you\b"),
    ("smalltalk.what_model", r"\bwhat (model|llm) are you\b|\bwhich model\b"),
    ("smalltalk.name", r"\bhow do (you|i) (say|pronounce)\b.*\b(aien|your name)\b|\bpronounce\b"),
    ("smalltalk.is_it_free", r"\b(is (it|this|aien) free|how much (does it )?cost|pricing|price)\b"),
    ("smalltalk.how_are_you", r"\bhow are you\b|\bhow'?s it going\b|\bwhat'?s up\b"),
    ("smalltalk.who_are_you", r"\bwho are you\b|\bwhat are you\b"),
    ("donation.how", r"\b(donat\w*|tip|support drake|sponsor\w*|chip in)\b"),
    ("encouragement.vibe_coder", r"\bvibe ?cod\w*\b|\bcan'?t (really )?code\b|\bnot a (real )?(coder|programmer)\b"),
    ("encouragement.stuck", r"\b(i'?m stuck|stuck on|frustrat\w*|giving up|give up|nothing works)\b"),
    ("academy.explain", r"\b(agent academy|academy|train (my|your) agent)\b"),
    ("voice.help", r"\b(microphone|mic|push to talk|talk instead|voice input|speak to you)\b"),
    ("mobile.home_screen", r"\b(home ?screen|on my phone|install (it|this|aien) on (my )?(phone|iphone|android))\b"),
    ("repo.how", r"github\.com/[\w.-]+/[\w.-]+|\b(look at|review|check out) my (repo|code|project)\b"),
    ("goodbye.general", r"^\s*(bye|goodbye|good night|see (you|ya)|later|cya|gotta go|i'?m off)\b"),
    ("smalltalk.thanks", r"^\s*(thanks|thank you|thx|ty|cheers|appreciate it)\b"),
]
INTENTS = [(s, re.compile(p, re.I)) for s, p in INTENTS]

GREETING = re.compile(r"^\s*(hi|hello|hey|hiya|yo|howdy|greetings|good (morning|afternoon|evening)|hi there|hey there|"
                      r"hello there)( (aien|allen))?[\s!.,]*$", re.I)


def time_situation(hour):
    if hour is None:
        return "greeting.first_visit"
    if 5 <= hour < 12:
        return "greeting.morning"
    if 12 <= hour < 17:
        return "greeting.afternoon"
    if 17 <= hour < 22:
        return "greeting.evening"
    return "greeting.late_night"


class Voice:
    def __init__(self, catalog_path, features=()):
        with open(catalog_path, encoding="utf-8") as f:
            cat = json.load(f)
        self.features = frozenset(features)
        self.clips = {c["id"]: c for c in cat["clips"]}
        self.by_situation = {}
        for c in cat["clips"]:
            if set(c.get("requires", ())) <= self.features:
                self.by_situation.setdefault(c["situation"], []).append(c["id"])

    def choose(self, situation, key, seen=()):
        ids = self.by_situation.get(situation)
        if not ids:
            return None
        fresh = [i for i in ids if i not in seen] or ids
        h = int.from_bytes(hashlib.sha256((situation + "\0" + key).encode()).digest()[:8], "big")
        return fresh[h % len(fresh)]

    def situation_for(self, user_text, turn=0, answer_id=None):
        if turn == 0 and GREETING.match(user_text or ""):
            return "greeting"
        if answer_id in ANSWER_SITUATION:
            return ANSWER_SITUATION[answer_id]
        for sit, pat in INTENTS:
            if pat.search(user_text or ""):
                return sit
        return None

    def pick(self, user_text="", situation=None, answer_id=None, turn=0, hour=None, returning=False, seen=()):
        """Clip ids to play in order for this turn; [] means text-only."""
        key = "%s\0%d" % (user_text or "", turn)
        sit = situation or self.situation_for(user_text, turn, answer_id)
        if sit is None:
            return []
        if sit == "greeting":
            first = "greeting.return_visit" if returning else time_situation(hour)
            out = [self.choose(first, key, seen) or self.choose("greeting.first_visit", key, seen),
                   self.choose("intro.menu", key, seen)]
            return [i for i in out if i]
        c = self.choose(sit, key, seen)
        return [c] if c else []

    def files(self, ids):
        """[{id, file, text, duration}] for the page player."""
        return [{k: self.clips[i][k] for k in ("id", "file", "text", "duration")} for i in ids if i in self.clips]

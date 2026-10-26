"""Tests for allen_voice against the real catalog. usage: python3 test_allen_voice.py CATALOG_JSON [VOICE_DIR]"""
import hashlib, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import allen_voice

CAT = sys.argv[1]
VDIR = sys.argv[2] if len(sys.argv) > 2 else os.path.dirname(CAT)
fails = 0


def check(name, cond):
    global fails
    print(("PASS " if cond else "FAIL ") + name)
    fails += not cond


v = allen_voice.Voice(CAT)
sit = lambda ids: [v.clips[i]["situation"] for i in ids]

check("morning greeting + menu", sit(v.pick("hey", hour=9)) == ["greeting.morning", "intro.menu"])
check("late night greeting", sit(v.pick("hello aien", hour=2))[0] == "greeting.late_night")
check("return visit greeting", sit(v.pick("hi", returning=True, hour=15))[0] == "greeting.return_visit")
check("greeting only on turn 0", v.pick("hi", turn=3) == [])
check("cached answer omega", sit(v.pick("what is omega?", answer_id="omega")) == ["explain.omega"])
check("are you allen", sit(v.pick("wait, are you Allen?", turn=2)) == ["smalltalk.are_you_allen"])
check("vibe coder", sit(v.pick("I can't code, I just vibe code, is this for me?", turn=1)) == ["encouragement.vibe_coder"])
check("donation", sit(v.pick("how can I donate?", turn=4)) == ["donation.how"])
check("thanks", sit(v.pick("thanks man", turn=5)) == ["smalltalk.thanks"])
check("bye", sit(v.pick("bye!", turn=6)) == ["goodbye.general"])
check("explicit situation", sit(v.pick(situation="error.busy")) == ["error.busy"])
check("no match -> text only", v.pick("explain the paged KV allocator in detail", turn=2) == [])
check("unknown situation -> text only", v.pick(situation="nope.none") == [])
check("deterministic", v.pick("hey", hour=9) == v.pick("hey", hour=9))
a = v.pick(situation="goodbye.general", user_text="x")
check("seen clips skipped", v.pick(situation="goodbye.general", user_text="x", seen=a) != a)
check("gated: repo clips off by default", v.pick("check out my repo github.com/a/b", turn=1) == [])
check("gated: menu never mentions repo when repo-coach off",
      all("repo" not in v.clips[i]["text"].lower() for i in v.by_situation["intro.menu"]))
v2 = allen_voice.Voice(CAT, features={"repo-coach", "voice-input"})
check("gated: repo clips on with feature", sit(v2.pick("github.com/a/b", turn=1)) == ["repo.how"])
check("gated: voice help on with feature", sit(v2.pick("can I use the microphone?", turn=1)) == ["voice.help"])
check("files() shape", set(v.files(v.pick("hey", hour=9))[0]) == {"id", "file", "text", "duration"})

cat = json.load(open(CAT))
bad = [c["id"] for c in cat["clips"]
       if hashlib.sha256(open(os.path.join(VDIR, os.path.basename(c["file"])), "rb").read()).hexdigest() != c["sha256"]]
check("every clip file matches its sha256 (%d clips)" % len(cat["clips"]), not bad)
check("every clip has license and source", all(c.get("license") and c.get("source", {}).get("voice_id") for c in cat["clips"]))
print("RESULT", "PASS" if not fails else "FAIL %d" % fails)
sys.exit(1 if fails else 0)

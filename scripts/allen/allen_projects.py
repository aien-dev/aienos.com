"""The build loop in the public gateway: homes, save, publish, gallery, and Drake's approve list.

allen_chat.py calls into this module from a few hook lines; it holds no GitHub credential.
Save and publish items go into the same write-only spool as issues and are filed by
omega-projects-pusher (allen_build.py documents the item shapes).

Public routes (/allen/api/...):
  POST projects/handle  {"name"}   remembered visitor (X-Allen-Key): the stable handle
                                   <name>-<8 hex>, remembered in the visitor Cortex as
                                   "Builder handle: ..."; anonymous: a one-time handle
                                   anon-<10 hex> and its claim code, shown once and never stored.
  GET  projects/me                 the caller's handle: from the visitor Cortex, or from the
                                   claim code in X-Allen-Claim; null when there is none.
  POST projects/save    item       one commit on u/<handle> (viewable on GitHub at once)
  POST projects/publish item       a pull request to main for Drake's approval
  GET  gallery                     merged projects ("Built with Allen")
Ownership is checked here, server side, for every save and publish: a remembered handle
must match HMAC(secret, vid) for the caller's own key, an anon- handle must match the
caller's claim code. Model output cannot set either header.

Console (Drake, tailnet only): a "Projects waiting for you" section listing open publish
requests with links to review and approve on GitHub, a Decline button (the pusher comments
and closes the pull request) and a Refresh button (the pusher rewrites the index).
"""

import html
import os
import secrets

import allen_build as build
import allen_spool as spool

NOTE = "Builder handle: "


def init(app, buckets):
    """Called from App.__init__ with the gateway's token-bucket class."""
    app.build_save_rate = buckets(10, 60)
    app.build_publish_rate = buckets(3, 1200)
    app.build_handle_rate = buckets(5, 600)


def _secret(h):
    return h.app.visitor_secret


def _claim(h):
    c = h.headers.get("X-Allen-Claim", "").strip()
    return c if build.CLAIM.match(c) else None


def get(h, route):
    """Public GET routes; True when handled."""
    if route == "projects/me":
        return _me(h) or True
    if route == "gallery":
        idx = build.read_index(h.app.args.spool_status) if h.app.args.spool_status else build.read_index("/nonexistent")
        h.send_json(200, {"updated": idx.get("updated"), "gallery": idx["gallery"],
                          "repo": build.REPO_URL})
        return True
    return False


def post(h, route):
    """Public POST routes; True when handled."""
    if route == "projects/handle":
        return _handle(h) or True
    if route in ("projects/save", "projects/publish"):
        return _drop(h, route.split("/")[1]) or True
    return False


def _me(h):
    app, secret = h.app, _secret(h)
    if not secret:
        return h.send_json(503, {"error": "unavailable", "message": "Homes are not available right now."})
    claim = _claim(h)
    if claim:
        return h.send_json(200, {"handle": build.handle_for_claim(secret, claim), "anonymous": True})
    vid = app.vid(h)
    handle = None
    if vid and app.memory:
        for note in app.memory.recall(vid, 40):
            if note.startswith(NOTE):
                cand = note[len(NOTE):].split()[0] if note[len(NOTE):].split() else ""
                if build.owns(secret, cand, vid=vid):
                    handle = cand
                    break
    return h.send_json(200, {"handle": handle, "anonymous": vid is None})


def _handle(h):
    app, secret = h.app, _secret(h)
    if not secret:
        return h.send_json(503, {"error": "unavailable", "message": "Homes are not available right now."})
    req = h.read_json()
    if req is None:
        return None
    retry = app.build_handle_rate.take(app.client_key(h))
    if retry:
        return h.refuse(429, "rate", "Too many requests. Try again shortly.", retry)
    vid = app.vid(h)
    if vid is None:
        claim = build.new_claim()
        app.stats.add("build_handle_anon")
        return h.send_json(200, {"handle": build.handle_for_claim(secret, claim), "claim": claim, "anonymous": True,
                                 "message": "Keep this claim code: it is shown once and is the only way to "
                                            "save to this home again."})
    name = req.get("name") if isinstance(req, dict) else None
    try:
        handle = build.handle_for_vid(secret, vid, name if isinstance(name, str) else "")
    except build.Invalid as e:
        return h.send_json(422, {"error": "invalid", "message": str(e)})
    remembered = bool(app.memory and app.memory.replace_note(vid, NOTE, handle, max_chars=80))
    app.stats.add("build_handle")
    return h.send_json(200, {"handle": handle, "anonymous": False, "remembered": remembered,
                             "home": "%s/tree/u/%s/projects/%s" % (build.REPO_URL, handle, handle)})


def _drop(h, kind):
    app, secret = h.app, _secret(h)
    app.stats.add("build_%s_requests" % kind)
    if not app.args.spool_in or not secret:
        return h.send_json(404, {"error": "not_found"})
    req = h.read_json()
    if req is None:
        return None
    if not isinstance(req, dict):
        return h.send_json(422, {"error": "invalid", "message": "Send a JSON object."})
    req = dict(req, v=1, kind=kind)
    if not build.owns(secret, req.get("handle"), vid=app.vid(h), claim=_claim(h)):
        app.stats.add("build_not_owner")
        return h.send_json(403, {"error": "not_owner",
                                 "message": "That home belongs to someone else, or your key or claim code is missing."})
    try:
        item = build.validate(req)
    except build.Invalid as e:
        app.stats.add("build_invalid")
        return h.send_json(422, {"error": "invalid", "message": str(e)})
    iid = spool.item_id(item)
    st = spool.read_status(app.args.spool_status, iid)
    if st is None and os.path.exists(os.path.join(app.args.spool_in, iid + ".json")):
        st = {"id": iid, "state": "queued", "url": None, "reason": None}
    if st is not None:
        return h.send_json(200, dict(st, duplicate=True, status="/allen/api/status/" + iid))
    rate = app.build_save_rate if kind == "save" else app.build_publish_rate
    retry = rate.take(app.client_key(h))
    if retry:
        return h.refuse(429, "rate", "You have saved several times already. Try again shortly.", retry)
    iid, _ = spool.write_item(app.args.spool_in, item)
    app.stats.add("build_" + kind)
    branch = "u/" + item["handle"] if kind == "save" else "p/%s/%s" % (item["handle"], item["project"])
    return h.send_json(200, {"id": iid, "state": "queued", "url": None, "reason": None, "duplicate": False,
                             "status": "/allen/api/status/" + iid, "branch": branch,
                             "view": "%s/tree/%s/%s" % (build.REPO_URL, branch, build.project_dir(item))})


# ---------------------------------------------------------------- Drake's console

def console_html(c, csrf):
    """The approve/decline list for the console dashboard."""
    app = c.app
    idx = build.read_index(app.args.spool_status) if app.args.spool_status else build.read_index("/nonexistent")
    esc = html.escape
    rows = []
    for o in idx["open"]:
        url = o.get("url", "")
        if not (isinstance(url, str) and url.startswith(build.REPO_URL + "/pull/")):
            continue
        rows.append("<tr><td><a href='%s'>#%d</a></td><td>%s</td><td>%s/%s</td>"
                    "<td><a href='%s/files'>Review and approve on GitHub</a></td>"
                    "<td><form class=inline method=post action=/projects/decline>%s"
                    "<input type=hidden name=pr value='%d'><input type=text name=reason maxlength=1000 "
                    "placeholder='Reason (shown on the pull request)'><button>Decline</button></form></td></tr>" % (
                        esc(url, quote=True), o["number"], esc(o.get("title", "")), esc(o.get("handle", "")),
                        esc(o.get("project", "")), esc(url, quote=True), c.csrf_field(csrf), o["number"]))
    table = ("<table><tr><th>Request</th><th>Title</th><th>Home</th><th>Approve</th><th>Decline</th></tr>%s</table>"
             % "".join(rows)) if rows else "<p>No projects are waiting.</p>"
    return ("<section><h2>Projects waiting for you</h2>%s<p>Index updated %s &middot; %d in the gallery.</p>"
            "<form method=post action=/projects/refresh>%s<button>Refresh from GitHub</button></form></section>") % (
        table, esc(str(idx.get("updated") or "never")), len(idx["gallery"]), c.csrf_field(csrf))


def console_post(c, path, form):
    """Console POST routes after login and CSRF checks; True when handled."""
    app = c.app
    if path not in ("/projects/decline", "/projects/refresh"):
        return False
    if not app.args.spool_in:
        c.send_body(404, b"no spool configured\n", "text/plain")
        return True
    if path == "/projects/decline":
        try:
            item = build.validate({"v": 1, "kind": "decline", "pr": int(form.get("pr", "")),
                                   "reason": " ".join(form.get("reason", "").split())})
        except (ValueError, build.Invalid) as e:
            c.send_body(422, ("cannot decline: %s\n" % e).encode(), "text/plain")
            return True
    else:
        item = {"v": 1, "kind": "refresh", "nonce": secrets.token_hex(8)}
    spool.write_item(app.args.spool_in, item)
    app.stats.add("console_" + item["kind"])
    c.redirect("/")
    return True

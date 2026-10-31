#!/usr/bin/env python3
"""A2A protocol 1.0 JSON-RPC for one call: SendMessage, passed to the introduce door.

The HTTP route is POST /allen/api/a2a (public POST /aien/api/a2a). This module
does not write a file, a log line, or a task. The only store is the introduce
door, and only when that door accepts the record. Consent is the introduce
door's rule: the JSON boolean true. Anything else stores nothing. The handler
sends Cache-Control: no-store, the same header as introduce.

Supported method:
  SendMessage. The legacy method name message/send is the same call.

Not supported. A known method returns the A2A error for the missing capability.
Any other method returns JSON-RPC -32601 Method not found. Nothing is stored.

  SendStreamingMessage, message/stream: -32004 Unsupported operation.
    capabilities.streaming is false.
  SubscribeToTask, tasks/resubscribe: -32004 Unsupported operation.
    capabilities.streaming is false.
  GetTask, tasks/get: -32004 Unsupported operation. This agent keeps no tasks.
  ListTasks, tasks/list: -32004 Unsupported operation. This agent keeps no tasks.
  CancelTask, tasks/cancel: -32004 Unsupported operation. This agent keeps no tasks.
  CreateTaskPushNotificationConfig, tasks/pushNotificationConfig/set: -32003
    Push notifications are not supported. capabilities.pushNotifications is false.
  GetTaskPushNotificationConfig, tasks/pushNotificationConfig/get: -32003.
  ListTaskPushNotificationConfigs, tasks/pushNotificationConfig/list: -32003.
  DeleteTaskPushNotificationConfig, tasks/pushNotificationConfig/delete: -32003.
  GetExtendedAgentCard, agent/getAuthenticatedExtendedCard: -32004
    Unsupported operation. capabilities.extendedAgentCard is false.

A2A-Version: spec 3.6.2 says an empty value is version 0.3, and spec 3.6.1
assumes 0.3 when the header is empty. A missing header is that empty case.
This door implements 1.0 only, so a missing or empty version is 0.3 and
returns -32009, and nothing is stored. Send A2A-Version: 1.0. When the header
is absent, the query parameter A2A-Version is read instead (spec 3.6.1). Any
other version returns -32009. Notifications (a request with no id) are
rejected with -32600. This door does not take a silent store.

SendMessage params.message is the introduction. Fields come from message.metadata,
then from each JSON data part (a later part overrides). A text part is the
purpose only when purpose is still empty. role must be ROLE_USER. File parts
(raw, url) return -32005. taskId and a non-empty referenceTaskIds return -32004.
historyLength, returnImmediately, extensions, and the A2A-Extensions header are
ignored and are not stored. The JSON-RPC envelope, messageId, and contextId are
not stored. contextId is echoed when the client sent one, otherwise a new id is
returned and forgotten.

The introduce door still applies its own caps, the anonymous-mode refusal, the
rate limit, and the daily cap. Those replies are JSON-RPC errors here. HTTP
status is 200 for every JSON-RPC body, including errors, with the error in the
body. Cache-Control stays no-store.
"""

import json
import secrets
from urllib.parse import parse_qs, urlsplit

# The introduce door's field names. consent is checked by that door, not copied
# into the stored record.
INTRO_FIELDS = ("kind", "name", "org", "contact", "github", "origin", "purpose",
                "homepage", "framework", "model", "human")

SEND_METHODS = frozenset({"SendMessage", "message/send"})

# code, message, reason. Legacy slash names are the same operations.
UNSUPPORTED = {
    "SendStreamingMessage": (-32004, "Unsupported operation", "STREAMING_NOT_SUPPORTED"),
    "message/stream": (-32004, "Unsupported operation", "STREAMING_NOT_SUPPORTED"),
    "SubscribeToTask": (-32004, "Unsupported operation", "STREAMING_NOT_SUPPORTED"),
    "tasks/resubscribe": (-32004, "Unsupported operation", "STREAMING_NOT_SUPPORTED"),
    "GetTask": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "tasks/get": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "ListTasks": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "tasks/list": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "CancelTask": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "tasks/cancel": (-32004, "Unsupported operation", "TASKS_NOT_SUPPORTED"),
    "CreateTaskPushNotificationConfig": (-32003, "Push notifications are not supported",
                                         "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "tasks/pushNotificationConfig/set": (-32003, "Push notifications are not supported",
                                         "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "GetTaskPushNotificationConfig": (-32003, "Push notifications are not supported",
                                      "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "tasks/pushNotificationConfig/get": (-32003, "Push notifications are not supported",
                                         "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "ListTaskPushNotificationConfigs": (-32003, "Push notifications are not supported",
                                        "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "tasks/pushNotificationConfig/list": (-32003, "Push notifications are not supported",
                                          "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "DeleteTaskPushNotificationConfig": (-32003, "Push notifications are not supported",
                                         "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "tasks/pushNotificationConfig/delete": (-32003, "Push notifications are not supported",
                                            "PUSH_NOTIFICATIONS_NOT_SUPPORTED"),
    "GetExtendedAgentCard": (-32004, "Unsupported operation", "EXTENDED_AGENT_CARD_NOT_SUPPORTED"),
    "agent/getAuthenticatedExtendedCard": (-32004, "Unsupported operation",
                                           "EXTENDED_AGENT_CARD_NOT_SUPPORTED"),
}

OUTPUT_MODES = frozenset({"text/plain", "application/json"})
MAX_PARTS = 8
MAX_ID = 200


def error_info(reason, metadata=None):
    info = {
        "@type": "type.googleapis.com/google.rpc.ErrorInfo",
        "reason": reason,
        "domain": "a2a-protocol.org",
    }
    if metadata:
        info["metadata"] = {str(k): str(v) for k, v in metadata.items()}
    return [info]


def bad_request(field, description):
    return [{
        "@type": "type.googleapis.com/google.rpc.BadRequest",
        "fieldViolations": [{"field": field, "description": description}],
    }]


def error_obj(rid, code, message, data=None):
    err = {"code": code, "message": message}
    if data is not None:
        err["data"] = data
    return {"jsonrpc": "2.0", "id": rid, "error": err}


def valid_id(rid):
    if rid is None or isinstance(rid, str):
        return True
    # bool is a subclass of int. JSON-RPC ids are strings, numbers, or null.
    if type(rid) is bool:
        return False
    return type(rid) in (int, float)


def controls(text):
    return any((ord(c) < 32 and c not in "\n\t") or ord(c) == 127 for c in text)


def _header(headers, name):
    """(value, present). Missing is not the same as an empty value."""
    if headers is None:
        return None, False
    for key in headers.keys():
        if str(key).lower() == name.lower():
            return headers.get(key), True
    return None, False


def version_token(headers, path=""):
    """The version this door must apply.

    Spec 3.6.2: an empty value is 0.3. Spec 3.6.1: an empty header is assumed
    to be 0.3, and a client may send A2A-Version as a query parameter instead
    of the header. A missing header is that empty case, so it is 0.3 unless
    the query parameter is present. This door speaks 1.0 only.
    """
    raw, present = _header(headers, "A2A-Version")
    if not present and path:
        query = parse_qs(urlsplit(path).query, keep_blank_values=True)
        if "A2A-Version" in query:
            raw = query["A2A-Version"][0] if query["A2A-Version"] else ""
            present = True
    if not present or raw is None or str(raw).strip() == "":
        return "0.3"
    return str(raw).strip()


def version_error(headers, path=""):
    """None when the call is version 1.0. Otherwise VersionNotSupportedError."""
    version = version_token(headers, path)
    if version == "1.0":
        return None
    return (-32009, "Version is not supported",
            error_info("VERSION_NOT_SUPPORTED", {"version": version}))


def parse(raw):
    """('ok', request) or ('error', response object)."""
    if not isinstance(raw, (bytes, bytearray)):
        return "error", error_obj(None, -32700, "Invalid JSON payload")
    try:
        req = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, ValueError):
        return "error", error_obj(None, -32700, "Invalid JSON payload")
    if not isinstance(req, dict):
        return "error", error_obj(None, -32600, "Request payload validation error")
    rid = req.get("id", None)
    if "id" not in req or not valid_id(rid):
        # A notification, or an id this door will not echo. No silent store.
        return "error", error_obj(None, -32600, "Request payload validation error")
    if req.get("jsonrpc") != "2.0" or not isinstance(req.get("method"), str) or not req["method"]:
        return "error", error_obj(rid, -32600, "Request payload validation error")
    if "params" in req and not isinstance(req["params"], dict):
        return "error", error_obj(rid, -32602, "Invalid parameters",
                                  bad_request("params", "params must be an object"))
    return "ok", req


def check_config(params):
    cfg = params.get("configuration")
    if cfg is None:
        return None
    if not isinstance(cfg, dict):
        return (-32602, "Invalid parameters", bad_request("configuration", "configuration must be an object"))
    push = cfg.get("taskPushNotificationConfig")
    if push is not None:
        return (-32003, "Push notifications are not supported",
                error_info("PUSH_NOTIFICATIONS_NOT_SUPPORTED"))
    modes = cfg.get("acceptedOutputModes")
    if modes is None:
        return None
    if not isinstance(modes, list) or not modes or not all(isinstance(m, str) for m in modes):
        return (-32602, "Invalid parameters",
                bad_request("configuration.acceptedOutputModes", "acceptedOutputModes must be media types"))
    if OUTPUT_MODES.isdisjoint(modes):
        return (-32005, "Content type is not supported",
                error_info("CONTENT_TYPE_NOT_SUPPORTED", {"acceptedOutputModes": ",".join(modes)}))
    return None


def build_intro(message):
    """('ok', intro dict, context id) or ('error', code, message, data)."""
    if not isinstance(message, dict):
        return ("error", -32602, "Invalid parameters",
                bad_request("message", "message is required"))
    if message.get("role") != "ROLE_USER":
        return ("error", -32602, "Invalid parameters",
                bad_request("message.role", "role must be ROLE_USER"))
    mid = message.get("messageId")
    if not isinstance(mid, str) or not mid.strip() or len(mid) > MAX_ID or controls(mid):
        return ("error", -32602, "Invalid parameters",
                bad_request("message.messageId", "messageId must be text"))
    if message.get("taskId") or message.get("referenceTaskIds"):
        return ("error", -32004, "Unsupported operation",
                error_info("TASKS_NOT_SUPPORTED"))
    ctx = message.get("contextId")
    if ctx is None:
        context_id = "ctx_" + secrets.token_hex(16)
    elif not isinstance(ctx, str) or not ctx.strip() or len(ctx) > MAX_ID or controls(ctx):
        return ("error", -32602, "Invalid parameters",
                bad_request("message.contextId", "contextId must be text"))
    else:
        context_id = ctx
    parts = message.get("parts")
    if not isinstance(parts, list) or not parts or len(parts) > MAX_PARTS:
        return ("error", -32602, "Invalid parameters",
                bad_request("message.parts", "Send one to eight parts"))
    meta = message.get("metadata", {})
    if meta is None:
        meta = {}
    if not isinstance(meta, dict):
        return ("error", -32602, "Invalid parameters",
                bad_request("message.metadata", "metadata must be an object"))
    intro = dict(meta)
    texts = []
    for part in parts:
        if not isinstance(part, dict):
            return ("error", -32602, "Invalid parameters",
                    bad_request("message.parts", "each part must be an object"))
        present = [k for k in ("text", "raw", "url", "data") if k in part]
        if len(present) != 1:
            return ("error", -32602, "Invalid parameters",
                    bad_request("message.parts", "a part needs exactly one of text, raw, url, data"))
        kind = present[0]
        media = part.get("mediaType")
        if kind in ("raw", "url"):
            return ("error", -32005, "Content type is not supported",
                    error_info("CONTENT_TYPE_NOT_SUPPORTED"))
        if kind == "text":
            if not isinstance(part["text"], str):
                return ("error", -32602, "Invalid parameters",
                        bad_request("message.parts", "text must be a string"))
            if media is not None and media != "text/plain":
                return ("error", -32005, "Content type is not supported",
                        error_info("CONTENT_TYPE_NOT_SUPPORTED", {"mediaType": media}))
            if part["text"].strip():
                texts.append(part["text"].strip())
        else:
            if media is not None and media != "application/json":
                return ("error", -32005, "Content type is not supported",
                        error_info("CONTENT_TYPE_NOT_SUPPORTED", {"mediaType": media}))
            if not isinstance(part["data"], dict):
                return ("error", -32602, "Invalid parameters",
                        bad_request("message.parts", "a data part must be a JSON object"))
            intro.update(part["data"])
    purpose = intro.get("purpose")
    if texts and (not isinstance(purpose, str) or not purpose.strip()):
        intro["purpose"] = "\n".join(texts)
    return "ok", intro, context_id


def map_intro(status, body):
    err = body.get("error") if isinstance(body, dict) else None
    msg = body.get("message") if isinstance(body, dict) and isinstance(body.get("message"), str) else "Invalid parameters"
    if err == "consent_required":
        return -32602, "Invalid parameters", bad_request("message.metadata.consent", msg)
    if err in ("invalid", "too_long"):
        return -32602, "Invalid parameters", bad_request("message", msg)
    if err == "anonymous":
        return -32602, "Invalid parameters", bad_request("X-Allen-Anonymous", msg)
    # Spec 3.3.2 names "rate limit exceeded" as an example of -32603. The
    # -32001 to -32009 table has no rate-limit code, so this stays -32603.
    if err == "rate":
        return -32603, "Internal error", error_info("RATE_LIMITED", {"message": msg})
    if err == "busy":
        return -32603, "Internal error", error_info("BUSY", {"message": msg})
    if err == "not_found":
        return -32603, "Internal error", error_info("INTRODUCTION_DOOR_UNAVAILABLE")
    return -32603, "Internal error", error_info("INTRODUCTION_FAILED", {"status": status})


def result_message(body, context_id):
    data = {k: body[k] for k in ("id", "stored", "delete", "key") if k in body}
    return {"message": {
        "role": "ROLE_AGENT",
        "messageId": "m_" + secrets.token_hex(16),
        "contextId": context_id,
        "parts": [
            {"text": "Introduction stored.", "mediaType": "text/plain"},
            {"data": data, "mediaType": "application/json"},
        ],
    }}


def dispatch(raw, headers, accept, path=""):
    """One JSON-RPC call. accept(intro) is the introduce door.

    Returns (http_status, object, extra_headers). This function does not write
    storage. accept does, and only when the introduce door stores a record.
    http_status is 200. The handler adds Cache-Control: no-store.
    """
    kind, parsed = parse(raw)
    if kind == "error":
        return 200, parsed, None
    ver = version_error(headers, path)
    if ver:
        return 200, error_obj(parsed["id"], ver[0], ver[1], ver[2]), None
    method = parsed["method"]
    if method not in SEND_METHODS:
        if method in UNSUPPORTED:
            code, message, reason = UNSUPPORTED[method]
            return 200, error_obj(parsed["id"], code, message,
                                  error_info(reason, {"method": method})), None
        return 200, error_obj(parsed["id"], -32601, "Method not found",
                              error_info("METHOD_NOT_FOUND",
                                         {"method": method, "supported": "SendMessage"})), None
    params = parsed.get("params") or {}
    cfg = check_config(params)
    if cfg:
        return 200, error_obj(parsed["id"], cfg[0], cfg[1], cfg[2]), None
    built = build_intro(params.get("message"))
    if built[0] == "error":
        return 200, error_obj(parsed["id"], built[1], built[2], built[3]), None
    status, body, retry = accept(built[1])
    extra = {"Retry-After": str(retry)} if retry else None
    if status == 200 and isinstance(body, dict) and body.get("stored") is True:
        return 200, {"jsonrpc": "2.0", "id": parsed["id"], "result": result_message(body, built[2])}, extra
    code, message, data = map_intro(status, body)
    return 200, error_obj(parsed["id"], code, message, data), extra

"""ElevenLabs for the announcer's voice pipeline (EH9, #225), after Highfall's
tools/voice/el.py.

This is the only module that reads the API key. It comes from the Windows
*user* environment (HKCU\\Environment, ELEVENLABS_API_KEY), falling back to the
process environment as Highfall's does. It is never printed, logged or
written anywhere: only the xi-api-key header carries it, and no error message
includes a request's headers.

Every call that spends credits runs inside `Ledger.batch`, which reads the
subscription's character_count before and after and appends the difference to
the ledger file, so the recorded numbers are the vendor's, not estimates.
"""

import base64
import contextlib
import datetime
import json
import os
import pathlib
import time
import urllib.error
import urllib.request

API = "https://api.elevenlabs.io"
SAMPLE_RATE = 24000  # pcm_24000: the highest PCM rate the Starter tier gets (Highfall, #85)
FORMAT = f"pcm_{SAMPLE_RATE}"


def _key():
    try:
        import winreg

        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as k:
            value = winreg.QueryValueEx(k, "ELEVENLABS_API_KEY")[0]
            if value:
                return value
    except OSError:
        pass
    except ImportError:
        pass
    value = os.environ.get("ELEVENLABS_API_KEY")
    if not value:
        raise SystemExit("ELEVENLABS_API_KEY is not set in the user environment")
    return value


class RateLimited(Exception):
    pass


def request(method, path, body=None, raw=False):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(API + path, data=data, method=method)
    req.add_header("xi-api-key", _key())
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            payload = r.read()
    except urllib.error.HTTPError as e:
        if e.code == 429 and method == "GET":
            raise RateLimited(path)
        raise SystemExit(f"{method} {path}: HTTP {e.code}: {e.read().decode(errors='replace')[:800]}")
    except urllib.error.URLError as e:
        raise SystemExit(f"{method} {path}: {e.reason}")
    return payload if raw else json.loads(payload)


def subscription():
    for wait in (5, 10, 20, 40, 60):
        try:
            s = request("GET", "/v1/user/subscription")
            break
        except RateLimited:
            time.sleep(wait)
    else:
        s = request("GET", "/v1/user/subscription")
    return {k: s.get(k) for k in ("tier", "character_count", "character_limit", "next_character_count_reset_unix")}


def tts(voice_id, body):
    """Text to speech: 16-bit mono PCM bytes at SAMPLE_RATE."""
    return request("POST", f"/v1/text-to-speech/{voice_id}?output_format={FORMAT}", body, raw=True)


def design(body):
    """One Voice Design round: [(16-bit mono PCM bytes, generated_voice_id, duration_secs)]."""
    r = request("POST", f"/v1/text-to-voice/design?output_format={FORMAT}", body)
    return [(base64.b64decode(p["audio_base_64"]), p["generated_voice_id"], p.get("duration_secs"))
            for p in r["previews"]], r.get("text")


def save_voice(body):
    """Keep a designed preview as a voice in the library (free): its voice id."""
    return request("POST", "/v1/text-to-voice", body)["voice_id"]


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


class Ledger:
    def __init__(self, path, read=None, sleep=None, log=print):
        self.path = pathlib.Path(path)
        self.read = read or (lambda: subscription())
        self.sleep = sleep or (lambda s: time.sleep(s))
        self.log = log

    def entries(self):
        if not self.path.exists():
            return []
        return json.loads(self.path.read_text(encoding="utf-8"))

    @contextlib.contextmanager
    def batch(self, name, note=""):
        before = self.read()
        record = {"batch": name, "note": note, "started": now(), "before": before["character_count"]}
        try:
            yield record
        finally:
            # The counter lags a call or more behind, so wait until it moves and then holds still.
            after = self.read()
            changed = after["character_count"] != before["character_count"]
            for _ in range(8):
                self.sleep(5)
                again = self.read()
                if again["character_count"] == after["character_count"] and changed:
                    break
                changed = changed or again["character_count"] != before["character_count"]
                after = again
            record["after"] = after["character_count"]
            record["credits"] = after["character_count"] - before["character_count"]
            record["limit"] = after["character_limit"]
            record["finished"] = now()
            entries = self.entries()
            entries.append(record)
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self.path.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
            self.log(f"[ledger] {name}: {record['credits']} credits "
                     f"({before['character_count']} -> {after['character_count']} of {after['character_limit']})")

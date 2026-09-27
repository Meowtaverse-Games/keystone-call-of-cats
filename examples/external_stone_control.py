#!/usr/bin/env python3
"""Standard-library example for the loopback external stone-control API."""
import json, os, time, urllib.request

TOKEN = os.environ["KEYSTONE_EXTERNAL_CONTROL_TOKEN"]
BASE = "http://127.0.0.1:38473/v1"
def request(path, method="GET", body=None):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(BASE + path, data=data, method=method,
        headers={"Authorization": "Bearer " + TOKEN, "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=3) as response: return json.load(response)

def wait_for(action):
    while True:
        action = request("/actions/" + str(action["id"]))
        if action["status"] in ("complete", "blocked", "rejected"):
            return action
        time.sleep(0.05)

start = wait_for(request("/session/start", "POST"))
if start["status"] != "complete":
    raise RuntimeError(start)
try:
    # Read after start so the generation comes from the active external session.
    state = request("/state")
    stone = state["stones"][0]["index"]
    action = request(f"/stones/{stone}/commands", "POST", {"generation": state["generation"], "command": "move", "direction": "right"})
    print(wait_for(action))
finally:
    request("/session/stop", "POST")

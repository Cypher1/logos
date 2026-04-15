import json

import requests

open = False
resp = requests.get("https://ntfy.sh/ellie_logos/json", stream=True)
for line in resp.iter_lines():
    obj = json.loads(line)
    message: str = str(obj.get("message"))
    event = obj["event"]

    if event == "open":
        open = True
        # success
        continue
    if event == "keepalive":
        continue
    if event == "message":
        if ": " in message:
            role, message = message.split(": ", 2)
        else:
            role = "user"
        print(role)
        print(message)
    else:
        # Raw message
        print(f"Unknown message (open = {open})", obj)

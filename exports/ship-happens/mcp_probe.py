#!/usr/bin/env python3
"""
mcp_probe.py — a minimal MCP client, so you can demo the protocol without
depending on venue wifi or someone else's app.

    python3 mcp_probe.py "where did I look for hiking clothes"
    python3 mcp_probe.py --tools

It starts `evo_server.py mcp` as a child process, performs the real
handshake, lists the tools, calls find_work, and prints both directions of
the exchange so a person watching can see it happen.

Standard library only.
"""
import json
import subprocess
import sys
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "evo_server.py")

DIM = "\033[2m"
BOLD = "\033[1m"
ACC = "\033[38;5;173m"
OFF = "\033[0m"


def send(p, msg, label):
    print(f"{DIM}  -> {label}{OFF}")
    p.stdin.write(json.dumps(msg) + "\n")
    p.stdin.flush()


def read(p):
    line = p.stdout.readline()
    if not line:
        return None
    return json.loads(line)


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 0

    q = " ".join(args)
    proc = subprocess.Popen(
        [sys.executable, SERVER, "mcp"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        text=True, bufsize=1,
    )

    print(f"\n{BOLD}evo boundary · mcp handshake{OFF}\n")

    send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                           "clientInfo": {"name": "mcp_probe", "version": "0.1"}}},
         "initialize")
    r = read(proc)
    info = r["result"]["serverInfo"]
    print(f"{ACC}  <- {info['name']} {info['version']}{OFF}")

    send(proc, {"jsonrpc": "2.0", "method": "notifications/initialized"}, "initialized")

    send(proc, {"jsonrpc": "2.0", "id": 2, "method": "tools/list"}, "tools/list")
    r = read(proc)
    names = [t["name"] for t in r["result"]["tools"]]
    print(f"{ACC}  <- {', '.join(names)}{OFF}")

    if q == "--tools":
        proc.stdin.close()
        proc.wait(timeout=5)
        print()
        return 0

    send(proc, {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": {"name": "find_work", "arguments": {"query": q}}},
         f'find_work("{q}")')
    r = read(proc)
    text = r["result"]["content"][0]["text"]

    print(f"\n{BOLD}  answer{OFF}\n")
    for line in text.split("\n"):
        print(f"    {line}")
    print()

    proc.stdin.close()
    proc.wait(timeout=5)
    return 0


if __name__ == "__main__":
    sys.exit(main())

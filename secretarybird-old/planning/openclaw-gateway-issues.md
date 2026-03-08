# OpenClaw Gateway Issues on WSL

## The Problem

The OpenClaw gateway frequently gets stuck in a state where it can't be started or stopped. This is the **#1 runtime issue** with the app and a major hurdle for non-technical users.

### Error cycle

```
$ openclaw gateway
Gateway failed to start: gateway already running (pid XXXX); lock timeout after 5000ms
Port 18789 is already in use.
Tip: openclaw gateway stop

$ openclaw gateway stop
Gateway service check failed: Error: systemctl --user unavailable:
Failed to connect to bus: No such file or directory

$ systemctl --user stop openclaw-gateway.service
Failed to connect to bus: No such file or directory
```

**The user is completely stuck.** The gateway is running, they can't stop it through the official commands, and they can't start a new one.

### Root cause

1. **`openclaw gateway stop` relies on systemd**, which doesn't exist in WSL by default. WSL uses init, not systemd, so `systemctl --user` fails with "No such file or directory".

2. **OpenClaw uses a lock file** at `/tmp/openclaw-<UID>/gateway.<hash>.lock` containing the PID of the running gateway. If the gateway process dies without cleaning up (crash, `kill -9`, WSL restart), the lock file becomes stale.

3. **Even after killing the process**, a new gateway may have already spawned on the port (e.g., if the Electron app's auto-start kicked in), creating a new PID that you also need to kill.

### Lock file location

```
/tmp/openclaw-1000/gateway.c3f49dd0.lock
```

Contents:
```json
{"pid": 4592, "createdAt": "2026-03-07T23:15:31.231Z", "configPath": "/home/user/.openclaw/openclaw.json", "startTime": 967920}
```

The `1000` is the user's UID. The hash (`c3f49dd0`) appears to be derived from the config path.

## Workarounds

### Manual fix (for developers)

```bash
# 1. Kill all OpenClaw processes
pkill -f openclaw-gateway
pkill -f openclaw

# 2. Remove the stale lock file
rm -f /tmp/openclaw-*/gateway.*.lock

# 3. Now you can start fresh
openclaw gateway
```

### In-app fix (STOP button)

The app has a STOP button that runs `pkill -f openclaw-gateway; pkill -f openclaw` via WSL. But it does **not** currently remove the stale lock file, which means:
- STOP kills the process
- But the lock file remains
- Next auto-start attempt may fail because the lock file says the gateway is "still running"

**Fix needed:** The `forceKillOpenClaw()` function in `electron/src/main.ts` should also delete the lock file after killing the process.

### Enable systemd in WSL (permanent fix for `openclaw gateway stop`)

Add to `/etc/wsl.conf`:
```ini
[boot]
systemd=true
```

Then restart WSL (`wsl --shutdown` from PowerShell). This makes `systemctl --user` work, which means `openclaw gateway stop` will work. However:
- Requires admin access to edit `/etc/wsl.conf`
- The installer would need to configure this
- Not all WSL distros support systemd

## Impact on adoption

This is a critical blocker for non-technical users:

1. **They won't know about `kill` or `pkill`** — these are developer tools
2. **The official OpenClaw commands don't work** — `openclaw gateway stop` fails on WSL
3. **The error message suggests commands that also fail** — `systemctl` doesn't work either
4. **The app becomes unusable** — if the gateway is stuck, the chat doesn't work, and the user has no way to fix it without terminal knowledge

## Recommended fixes

### Short-term (for hackathon)

1. Update `forceKillOpenClaw()` in `main.ts` to also remove the lock file:
   ```
   pkill -f openclaw-gateway; pkill -f openclaw; rm -f /tmp/openclaw-*/gateway.*.lock
   ```

2. Make the STOP button more prominent and add a tooltip explaining when to use it.

3. Add auto-recovery: if gateway start fails with "already running", automatically kill + remove lock + retry.

### Long-term

1. Enable systemd in WSL via the installer (`/etc/wsl.conf`)
2. Report the issue to OpenClaw — `openclaw gateway stop` should fall back to `kill <pid>` when systemd is unavailable, and clean up its own lock file
3. Consider using a port-based health check instead of relying on the lock file — if the port responds, the gateway is running; if not, clean up the lock and start fresh

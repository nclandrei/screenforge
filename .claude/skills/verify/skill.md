---
name: verify
description: Build screenforge and visually verify output in Chrome. Use after making changes to verify rendering.
---

# Screenforge Visual Verification

Verify screenforge output visually by building, running, and inspecting the HTML preview in Chrome.

## Step 1: Build and Run

```bash
cargo build --release 2>&1 && cargo run --release -- run --config ./screenforge.yaml 2>&1
```

If build fails, stop and report the error.

## Step 2: Ensure HTTP Server Running

Check if server is already running on port 8765, start only if needed:

```bash
lsof -i :8765 >/dev/null 2>&1 && echo "Server already running" || (cd /Users/anicolae/code/screenforge/output && python3 -m http.server 8765 &)
```

Wait 1 second for server startup if newly started.

## Step 3: Get Chrome Tab Context

```
mcp__claude-in-chrome__tabs_context_mcp({ createIfEmpty: true })
```

## Step 4: Find or Create Tab

Check if any existing tab is already at `http://localhost:8765`. If yes, use that tab. If not, create a new tab:

```
mcp__claude-in-chrome__tabs_create_mcp()
```

Then navigate to the preview:

```
mcp__claude-in-chrome__navigate({ tabId: <TAB_ID>, url: "http://localhost:8765" })
```

## Step 5: Screenshot and Verify

Take a screenshot:

```
mcp__claude-in-chrome__computer({ action: "screenshot", tabId: <TAB_ID> })
```

## Step 6: Visual Inspection

Analyze the screenshot for:

1. **Text rendering** - Text should be readable, not mirrored/flipped, properly positioned
2. **Phone mockup** - Device frame visible, screenshot inside phone, Dynamic Island rendered
3. **Background** - Gradient/pattern visible, colors correct
4. **Layout** - Elements properly positioned, no clipping or overflow

## Step 7: Report

**Status**:
- Pass: All elements render correctly
- Fail: Describe specific issues found

If issues found, describe what's wrong and suggest fixes.

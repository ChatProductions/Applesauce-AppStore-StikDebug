# Applesauce 0.4.0

**This release renames the app.** What was *touchHLE for iOS* is now
**Applesauce — a playful emulator for iOS**. The name, icon and identifier are
new; the emulation is the same as 0.3.0, plus the fixes below.

Applesauce is an unaffiliated fork. The emulation is entirely the work of
[touchHLE](https://github.com/touchHLE/touchHLE) and its fork
[HyperHLE](https://github.com/HyperHLE/HyperHLE), and neither project is
connected to this one or endorses it. Please report problems here rather than to
them.

## Read this first: your games do not move automatically

The bundle identifier changed, so Applesauce installs **alongside** the old app
with an empty library rather than replacing it.

1. Open Files and go to **On My iPhone**.
2. Find the old app's folder — *HyperHLE* if you had 0.2.0 or 0.3.0, *touchHLE*
   if you had 0.1.0.
3. Move `touchHLE_apps` from it into the **Applesauce** folder. Move
   `touchHLE_sandbox` too if you want your saves.
4. Delete the old app.

App-wide settings do not come across; set them again under Settings.

## New: iOS 15 and 16 support, via TrollStore

The deployment target drops from 17.4 to 15.0, and there are now three IPAs:

| Your iOS | Download | How JIT gets enabled |
| --- | --- | --- |
| 17.4 or newer | `Applesauce-iOS-unsigned.ipa` | Sideload, then StikDebug's bolt button |
| 15.0–17.0, TrollStore | `Applesauce-iOS-trollstore.ipa` | TrollStore's **Enable JIT**, each session |
| 15.0–17.0, TrollStore, **A11 or older** | `Applesauce-iOS-trollstore-permanent-jit.ipa` | Already on, and stays on |

The TrollStore builds carry the memory entitlements that let older hardware map
the emulator's 4GiB guest address space. A free Apple account cannot sign those,
which is why this is a TrollStore route rather than a sideloading one.

**The permanent-JIT build crashes on launch on A12 and newer** (iPhone XS/XR
onwards) — iOS 15 bans the entitlement it carries. Use the ordinary TrollStore
build there.

**iOS 15 and 16 are untested by this project.** The support is back-ported from
[nerivalaitis](https://github.com/nerivalaitis)' work, tested by them on iOS 15.
Reports from those versions are the most useful thing you can send.

## Fixes

- **A game no longer dies when a rotation is refused.** A guest asking to rotate
  while the host UI still has the old orientation pinned used to panic and take
  the whole game down mid-play; it now logs a warning and carries on.
- **Black screen after rotating, on games without a fullscreen layer.** SDL
  recreates its framebuffer object when the window leaves and re-enters
  fullscreen, and compositing kept using the one cached at startup.
- **A crash inside the emulator no longer wedges the app.** Panics were unwinding
  across the FFI boundary, leaving a black screen and a dead exit button with
  nothing in the log; they are now caught and reported.
- **Guest memory allocation falls back** instead of failing outright when iOS
  refuses the full read-write mapping, which is what blocked older devices.
- **JIT detection understands permanent JIT.** A TrollStore install with
  `dynamic-codesigning` was previously told it had no JIT, and had to use "Start
  Anyway" every time.
- **Portrait is selectable** under Settings → Starting Orientation, for games
  that declare landscape but draw portrait anyway.
- **The previous run's log is kept** as `touchhle-host-previous.log`, so
  force-quitting a hung game no longer destroys the log that recorded the hang.

Both emulator cores carry these fixes.

## Credits

- [touchHLE](https://github.com/touchHLE/touchHLE) — the original emulator.
- [HyperHLE](https://github.com/HyperHLE/HyperHLE) — the compatibility fork used
  as the default core.
- [nerivalaitis](https://github.com/nerivalaitis) — the iOS 15 back-port,
  TrollStore packaging and the memory-allocation fallback.

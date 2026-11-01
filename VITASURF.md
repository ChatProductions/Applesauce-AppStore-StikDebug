# VitaSurf — standalone PS Vita browser

This is the **VitaSurf development branch** of the repurposed Applesauce repository. The existing ios-host branch is preserved for recovery. Existing Applesauce code is legacy content and is **not** part of a working Vita browser.

## First step: browser engine feasibility

**Candidate: WPE WebKit + JavaScriptCore**, pending proof of ARMv7 32-bit compatibility, memory fit, and feasible Vita graphics/input backend. NetSurf is a low-resource fallback but lacks the modern browser-game feature set; Chromium is too heavy for the initial target.

Research gates, in order:
1. Audit WPE WebKit build dependencies and 32-bit ARM support, including JIT-less JavaScriptCore and WebAssembly availability.
2. Check VitaSDK toolchain compatibility, library dependencies, and memory budget.
3. Implement a minimal Vita platform backend only if the engine passes those checks.
4. Test local HTML/JavaScript, HTTPS, WebAssembly, Canvas, and WebGL separately.
5. Test unprotected H.264/HLS video with Vita hardware decoding; TLS Fixer is only relevant to Sony HTTP consumers.
6. Browser-only compatibility benchmarks: js-dos, GTA III, GTA Vice City, NFS Underground 2 Revived. These are test targets, not promised playable titles.

No remote rendering and no DRM. No VPK or engine build has been validated yet.

Upstream references:
- https://github.com/WebPlatformForEmbedded/WPEWebKit
- https://github.com/WebKit/WebKit
- https://github.com/netsurf-browser/netsurf
- https://github.com/devnoname120/vita_tls_fixer

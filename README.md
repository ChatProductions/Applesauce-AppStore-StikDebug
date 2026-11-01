# VitaSurf

Standalone PlayStation Vita browser research and development.

## Browser engine

**WPE WebKit + JavaScriptCore** is the first engine to evaluate, not yet a proven Vita port. Initial research must verify ARMv7 32-bit compilation, JIT-less JavaScript, WebAssembly, memory usage, and a Vita-compatible graphics backend before a VPK is promised.

## Goals

- Native, standalone PS Vita VPK; no remote rendering.
- Modern HTML, CSS, JavaScript and WebAssembly where feasible.
- HTTPS connectivity; evaluate compatibility with devnoname120's TLS Fixer when using Sony networking.
- Unprotected HTML5/H.264/HLS video playback.
- Browser game tests: js-dos, GTA III, GTA Vice City, NFS Underground 2 Revived.
- WebGL investigation; WebGPU is not an initial deliverable.
- DRM excluded.

## Current status — 2026-11-01

VitaSurf is **not yet a usable web browser**. Independent PS Vita hardware milestones:

- **Native display:** confirmed working on PS Vita (CDRAM-backed dual framebuffers).
- **QuickJS:** confirmed running natively on PS Vita; six JavaScript tests pass and rerun with changing input on Cross.
- **WebLoad HTTPS test:** `VitaSurf-WebLoad-Test.vpk` uses native Vita networking/SSL and attempts `https://example.com/`. It displays the HTTP status, bytes read, and basic HTML/title recognition. Hardware networking and certificate compatibility remain unverified.
- **WebKit/JavaScriptCore:** WebKit cross-compilation research is separate and incomplete; QuickJS is a working scripting proof of concept, not a complete DOM, HTML/CSS engine or WebAssembly solution.

GitHub Actions on the `ios-host` branch builds standalone diagnostic VPKs. The diagnostics use distinct Vita title IDs, so they can be tested independently.

## Project scope

VitaSurf targets modern HTML/CSS, JavaScript, DOM, WebAssembly, WebGL/WebGPU investigation, non-DRM video, audio, browsing and games on Vita hardware, subject to measurable platform limitations. **DRM is the only deliberately excluded feature.** No unimplemented capability is presented as working.


## References

- https://github.com/WebPlatformForEmbedded/WPEWebKit
- https://github.com/WebKit/WebKit
- https://github.com/netsurf-browser/netsurf
- https://github.com/devnoname120/vita_tls_fixer

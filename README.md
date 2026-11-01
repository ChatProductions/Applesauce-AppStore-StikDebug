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

## Current status

Engine feasibility stage only. No working browser or VPK has been built or verified.

## References

- https://github.com/WebPlatformForEmbedded/WPEWebKit
- https://github.com/WebKit/WebKit
- https://github.com/netsurf-browser/netsurf
- https://github.com/devnoname120/vita_tls_fixer

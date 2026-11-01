# Engine and media reconnaissance — 2026-11-01

## Findings verified from upstream documentation

1. Sony's official PS Vita OSS listing identifies the **Internet Browser** dependencies: **WebKit, Cairo, cURL, OpenSSL, ICU, FreeType, libxml2, PCRE, pixman, JPEG, zlib, expat**. This makes Sony's WebKit source release especially relevant for Vita-specific investigation. https://www.playstation.com/en-us/oss/ps-vita/
2. Sony publishes several PS Vita WebKit source snapshots, including firmware 3.61–3.65. These are LGPL source releases, **not** necessarily a complete independently buildable browser or a modern engine. https://www.playstation.com/en-us/oss/ps-vita/webkit/
3. WPE's own FAQ explicitly says **WPE currently only works on Linux-based operating systems**. Vita is not a Linux target. Therefore a WPE build with `-DPORT=WPE` in VitaSDK is *not* a realistic near-term expectation; it would require significant OS and graphics platform work. https://wpewebkit.org/about/faq.html
4. WPE 2.54's WPEPlatform layer uses system-specific display/buffer implementations, including Wayland, DRM/KMS and headless. None is a native Vita backend. https://wpewebkit.org/about/architecture.html
5. OpenNOW-vita demonstrates a standalone Vita hardware H.264 decoding path via `sceAvcdec`, direct SDL/GXM texture presentation, and Opus audio. It is a WebRTC streaming client, **not a browser**. https://github.com/OpenCloudGaming/OpenNOW-vita
6. VitaMediaDeck's `vita-hw-decoder` and `vita-https` projects may provide better reusable standalone media primitives. Check APIs and licenses before integration. https://github.com/spyro-98/VitaMediaDeck
7. FFmpeg-vita documents Vita-specific `h264_vita`, `aac_vita`, and direct VRAM-backed rendering. https://github.com/fish47/FFmpeg-vita

## Revised engineering decision

**Primary investigation path: Sony's Vita-era WebKit sources + VitaSDK-compatible WebKit/JavaScriptCore subset.** Use modern WPE WebKit as an API/architecture reference, **not** a drop-in Vita engine. This changes the earlier tentative WPE-first recommendation in light of its upstream Linux-only constraint.

### Immediate engineering gates

- Download and inventory Sony's 3.61–3.65 WebKit source snapshot, recording archive hash, contents, engine revision, licensing and build prerequisites.
- Identify which JavaScriptCore APIs and WebAssembly (if any) are present. Do not assume Vita-era WebKit supports WASM.
- Map Sony browser's published dependencies against VitaSDK-available ports and estimate RAM/binary footprint.
- Compare standalone media paths (`sceAvcdec`, `vita-hw-decoder`) separately from the DOM/JS engine.
- Only then add a real engine target to CMake and build it in GitHub Actions.

## Scope and limitations

- Browser games are **compatibility probes**, not guaranteed to run. WebGL and WASM require separate proof.
- NFS U2 Revived's WebGPU/shared-memory requirements make it an unlikely playable target.
- No DRM, no remote rendering, no redistribution of proprietary Sony binaries.
- No engine has been compiled or run on Vita yet. Existing SDL VPK is a smoke test only.

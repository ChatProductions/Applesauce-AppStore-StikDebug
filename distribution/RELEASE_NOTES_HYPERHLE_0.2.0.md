# touchHLE iOS Port 0.2.0 — HyperHLE core

An experimental, unofficial iOS port of touchHLE, now running the [HyperHLE](https://github.com/HyperHLE/HyperHLE) **v1.0.6** core instead of the touchHLE 0.2.3 core used by the previous iOS prerelease.

**This installs alongside 0.1.0 rather than replacing it.** It appears as **HyperHLE** on the home screen; the 0.1.0 build stays as **touchHLE**. The two cores support different sets of games and neither is strictly better — HyperHLE runs The Sims Medieval and fixes landscape input, but some games that work under touchHLE 0.2.3 do not work under it (Call of Duty: Zombies has been reported as one). Keep both installed and use whichever runs your game. HyperHLE is a fork of touchHLE with broader game compatibility; it has no iOS support of its own, so the host app, build scripts and packaging in this port are specific to it.

This is not an official release of touchHLE or of HyperHLE.

## Changes Since 0.1.0

- Now runs the HyperHLE v1.0.6 emulator core.
- **Landscape games accept touch input again.** The host handed the main thread to the emulator as soon as the scene reported it had rotated, but before UIKit had finished committing the rotation. The scene stayed mid-transition and UIKit stopped delivering touches to every window in it — games still rendered, and timers still ran, so nothing looked wrong.
- **Landscape games are the right way up.** The iOS OpenGL ES 2.0 direct presenter was rotating frames that were already in display orientation.
- **Resolution scaling no longer magnifies some games.** The scale hack enlarges renderbuffers but never textures, so games that render through an offscreen texture were drawn several times too large. Games affected by this previously showed only a magnified corner of the picture at any setting above 1x.

## Tested

Tested on an iPhone 16 Pro running iOS 27 beta 4 (build 24A5390f) with:

- Flappy Bird 1.1.0 — playable, including at 2x/3x/4x resolution scale.
- Touch & Go 1.1 — playable at all resolution settings.
- Tony Hawk's Pro Skater 2 1.2.1 — playable.
- Wolfenstein RPG 1.1.1 — playable, landscape and touch correct.
- Mirror's Edge 1.4.72 — playable.
- The Sims Medieval 1.0.1 — reaches gameplay, landscape and touch correct. **Known issue:** no keyboard appears for naming your Sim or your kingdom (see below).

The deployment target is iOS 17.4, but testing so far has only been on one device and only on an iOS **beta** — no shipping iOS release has been verified. Reports from stable iOS 17.4+ are especially useful. The [touchHLE compatibility database](https://appdb.touchhle.org/) is the best starting point for choosing an exact app version; its ratings do not guarantee a title has been tested through this iOS port, and HyperHLE's compatibility differs from touchHLE's.

## Known Issues

- The Sims Medieval: the on-screen keyboard does not appear when naming your Sim or your kingdom, so those names cannot be entered. The game is still playable past those screens: the confirm control sits near the top-right corner of the screen rather than where it is drawn, and tapping there advances you. The text field the game uses is not where it appears on screen, which is being looked into.
- Games rendering through an offscreen texture gain no extra detail from resolution scaling; the setting is applied only where it is safe to do so.
- Some games run under the touchHLE core but not HyperHLE. Call of Duty: Zombies has been reported working in 0.1.0 and not in 0.2.0. If a game fails here, try the 0.1.0 build before reporting it — and please include the exact app version.

## Requirements

- A modern iPhone running iOS 17.4 or newer.
- Installation through AltStore Classic, Xcode, or another compatible sideloading tool.
- JIT enabled for each new touchHLE process through StikDebug and LocalDevVPN, or AltJIT.
- A legally obtained, decrypted 32-bit IPA. Games are not included.

## Install With AltStore Classic

1. Install [AltStore Classic](https://altstore.io/) and complete its normal AltServer setup.
2. Download `touchHLE-HyperHLE-iOS-unsigned.ipa` from the **Assets** section below.
3. In AltStore, open **My Apps**, tap **+**, and select the downloaded IPA.
4. Let AltStore sign and install it using your own Apple account.
5. Enable JIT before starting a game.

Free Apple accounts normally require the app to be refreshed every seven days. A paid Apple Developer account provides longer-lived development signing, but it does not remove the JIT requirement.

## Install With Xcode

You can instead build and install the port from source with a free Personal Team or paid Apple Developer account. Choose your own signing team and bundle identifier; never publish your signed app, provisioning profile, certificate, team ID, or `xcuserdata`.

Follow the [complete Xcode build and installation instructions](https://github.com/johnny901901901/touchHLE-for-iOS/blob/ios-host/platform/ios/README.md#option-b-build-and-install-with-xcode).

## Enable JIT

Installing the app is not enough by itself. The Dynarmic CPU backend requires JIT, which must be enabled again whenever touchHLE starts as a new app process.

- Use the in-app bolt shortcut with [StikDebug](https://github.com/StephenDev0/StikDebug), LocalDevVPN, and your private device pairing file; or
- Use [AltJIT](https://faq.altstore.io/altstore-classic/altjit) through AltStore Classic.

Never share your pairing file. See the [complete JIT setup and troubleshooting guide](https://github.com/johnny901901901/touchHLE-for-iOS/blob/ios-host/platform/ios/README.md#enable-jit).

## Import A Game

1. Open touchHLE and tap **Import Game**.
2. Select a legally obtained decrypted 32-bit `.ipa` from Files.
3. Check the [touchHLE compatibility database](https://appdb.touchhle.org/) for the exact app version.
4. Enable JIT, then tap the imported game card.

Games are not included.

## Download Safety

The release IPA is unsigned and credential-free. Each user signs it locally with their own Apple account.

- File: `touchHLE-HyperHLE-iOS-unsigned.ipa`
- Size: `36192832` bytes
- SHA-256: `f68ac992f1f9627a0d98621316b9db4e3ee5d30e46a52fa0039daa38fd8685a2`

Do not install a copy that has been reuploaded with a different hash unless you trust and can verify the person who rebuilt it.

## Important

This build uses its own bundle identifier (`org.touchhle.ios.hyperhle`) and appears as **HyperHLE**, so it installs alongside the 0.1.0 **touchHLE** build instead of replacing it. Games and saves are not shared between the two — each keeps its own library.

If you already installed an earlier 0.2.0 that replaced your touchHLE app, reinstall 0.1.0 from its release to get the touchHLE build back.

## Credits

- [touchHLE](https://github.com/touchHLE/touchHLE) — the original emulator.
- [HyperHLE](https://github.com/HyperHLE/HyperHLE) — the compatibility fork used as this build's core.

The source remains subject to MPL-2.0 and the existing third-party licence requirements. Neither upstream project endorses this build.

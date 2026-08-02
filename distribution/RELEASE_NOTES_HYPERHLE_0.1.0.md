# touchHLE iOS Port — HyperHLE 0.1.0

An experimental, unofficial iOS port of touchHLE, now running the [HyperHLE](https://github.com/HyperHLE/HyperHLE) core (upstream commit `72cb2795`) instead of the touchHLE 0.2.3 core used by the previous iOS prerelease. HyperHLE is a fork of touchHLE with broader game compatibility; it has no iOS support of its own, so the host app, build scripts and packaging in this port are specific to it.

This is not an official release of touchHLE or of HyperHLE.

## Changes Since The 0.1.0 iOS Prerelease

- Now runs the HyperHLE emulator core.
- **Landscape games accept touch input again.** The host handed the main thread to the emulator as soon as the scene reported it had rotated, but before UIKit had finished committing the rotation. The scene stayed mid-transition and UIKit stopped delivering touches to every window in it — games still rendered, and timers still ran, so nothing looked wrong.
- **Landscape games are the right way up.** The iOS OpenGL ES 2.0 direct presenter was rotating frames that were already in display orientation.
- **Resolution scaling no longer magnifies some games.** The scale hack enlarges renderbuffers but never textures, so games that render through an offscreen texture were drawn several times too large. Games affected by this previously showed only a magnified corner of the picture at any setting above 1x.

## Tested

Tested on an iPhone 16 Pro (iOS 26) with:

- Flappy Bird — playable, including at 2x/3x/4x resolution scale.
- Touch & Go — playable at all resolution settings.
- Wolfenstein RPG — playable, landscape and touch correct.
- The Sims Medieval — reaches gameplay, landscape and touch correct. **Known issue:** the on-screen keyboard does not appear when naming your Sim, so a new game cannot be started past that point.

Tony Hawk's Pro Skater 2 was tested against the previous prerelease but has not been re-verified on this build.

The deployment target is iOS 17.4. Other device and iOS combinations still need wider testing. The [touchHLE compatibility database](https://appdb.touchhle.org/) is the best starting point for choosing an exact app version; its ratings do not guarantee a title has been tested through this iOS port, and HyperHLE's compatibility differs from touchHLE's.

## Known Issues

- The Sims Medieval: no keyboard for text entry (see above).
- Games rendering through an offscreen texture gain no extra detail from resolution scaling; the setting is applied only where it is safe to do so.

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

Follow the [complete Xcode build and installation instructions](https://github.com/johnny901901901/touchHLE/blob/ios-host/platform/ios/README.md#option-b-build-and-install-with-xcode).

## Enable JIT

Installing the app is not enough by itself. The Dynarmic CPU backend requires JIT, which must be enabled again whenever touchHLE starts as a new app process.

- Use the in-app bolt shortcut with [StikDebug](https://github.com/StephenDev0/StikDebug), LocalDevVPN, and your private device pairing file; or
- Use [AltJIT](https://faq.altstore.io/altstore-classic/altjit) through AltStore Classic.

Never share your pairing file. See the [complete JIT setup and troubleshooting guide](https://github.com/johnny901901901/touchHLE/blob/ios-host/platform/ios/README.md#enable-jit).

## Import A Game

1. Open touchHLE and tap **Import Game**.
2. Select a legally obtained decrypted 32-bit `.ipa` from Files.
3. Check the [touchHLE compatibility database](https://appdb.touchhle.org/) for the exact app version.
4. Enable JIT, then tap the imported game card.

Games are not included.

## Download Safety

The release IPA is unsigned and credential-free. Each user signs it locally with their own Apple account.

- File: `touchHLE-HyperHLE-iOS-unsigned.ipa`
- Size: `36191552` bytes
- SHA-256: `74b037cade25da7cba39b29766b0331d65958525e153360f9743cb048c295803`

Do not install a copy that has been reuploaded with a different hash unless you trust and can verify the person who rebuilt it.

## Important

This app installs under a different bundle identifier (`org.touchhle.ios.hyperhle`) from the earlier iOS prerelease, so both can be installed side by side. They do not share imported games or saves.

## Credits

- [touchHLE](https://github.com/touchHLE/touchHLE) — the original emulator.
- [HyperHLE](https://github.com/HyperHLE/HyperHLE) — the compatibility fork used as this build's core.

The source remains subject to MPL-2.0 and the existing third-party licence requirements. Neither upstream project endorses this build.

# touchHLE for iOS (Unofficial)

An experimental iOS port of [touchHLE](https://touchhle.org/), running the
[HyperHLE](https://github.com/HyperHLE/HyperHLE) v1.0.6 core. It plays supported
32-bit iPhone games on modern iPhones — **no jailbreak required**, though JIT is.

This is **not** an official release of touchHLE or of HyperHLE, and neither project
endorses it. The emulator core is their work; this repository adds the native iOS
host app, its build system and packaging. No games or Apple software are included.

| | |
| --- | --- |
| Port version | 0.2.0 |
| Emulator core | HyperHLE v1.0.6 (a fork of touchHLE) |
| Minimum iOS | 17.4 |
| Tested on | iPhone 16 Pro, iOS 27.0 |
| JIT | Required each time the app starts as a new process |
| Games | Not included — bring your own decrypted 32-bit IPA |

## Install

Download `touchHLE-HyperHLE-iOS-unsigned.ipa` from the
[latest release](https://github.com/johnny901901901/touchHLE/releases) and sideload
it with AltStore Classic, or build and sign it yourself in Xcode. Then enable JIT
with [StikDebug](https://github.com/StephenDev0/StikDebug) + LocalDevVPN, or AltJIT.

**[Full install, JIT, import and troubleshooting guide →](platform/ios/README.md)**

A free Apple account works, with Apple's usual seven-day signing limit. A paid
developer account signs for longer but does not remove the JIT requirement.

## What works

Tested on an iPhone 16 Pro running iOS 27.0. Exact app versions matter — the same
game can behave very differently between releases:

| Game | Version | Status |
| --- | --- | --- |
| Flappy Bird | 1.1.0 | Playable, including at 2x–4x resolution scale |
| Touch & Go | 1.1 | Playable at all resolution settings |
| Tony Hawk's Pro Skater 2 | 1.2.1 | Playable |
| Wolfenstein RPG | 1.1.1 | Playable |
| Mirror's Edge | 1.2.2 | Playable |
| The Sims Medieval | 1.0.1 | Playable, but see below |

Only one device and one iOS version have been tested so far. Wider testing is the
most useful thing anyone can contribute.

## Known issues

- **The Sims Medieval:** no keyboard appears when naming your Sim or your kingdom,
  so those names cannot be entered. The game is still playable past those screens —
  the confirm control sits near the top-right corner rather than where it is drawn.
- JIT is required; there is no interpreter fallback.
- Games rendering through an offscreen texture gain no extra detail from resolution
  scaling, which is applied only where it is safe to do so.
- A high rating in the [compatibility database](https://appdb.touchhle.org/)
  describes touchHLE generally and does not guarantee behaviour through this port.

## Reporting a game

[Open an issue](https://github.com/johnny901901901/touchHLE/issues/new/choose) using
the compatibility report form. Please include the exact app version, your iPhone
model and iOS version, whether JIT was enabled, the port version, and a log excerpt
(`touchHLE_log.txt`, reachable in Files under *On My iPhone → touchHLE*).

**Do not post IPA files or links to them, pairing files, signing certificates,
provisioning profiles or device backups.**

## Building

See [platform/ios/README.md](platform/ios/README.md#build-from-source). In short:

```sh
git clone --recurse-submodules https://github.com/johnny901901901/touchHLE.git
cd touchHLE
sh platform/ios/scripts/build-host.sh iphoneos Release
```

`vendor/dynarmic` points at a fork carrying the changes needed to run the arm64 JIT
inside a signed iOS app (MAP_JIT / W^X handling), so the `--recurse-submodules` part
matters.

## Credits

- [touchHLE](https://github.com/touchHLE/touchHLE) — the original emulator, and the
  work this rests on entirely.
- [HyperHLE](https://github.com/HyperHLE/HyperHLE) — the compatibility fork used as
  this build's core.
- [u/WorriedEquipment2241](https://www.reddit.com/user/WorriedEquipment2241/) for
  publicly demonstrating a separate touchHLE iOS experiment on a jailbroken device,
  which helped show the direction was worth pursuing.

Licensed under MPL-2.0, subject to the existing third-party licence requirements.
Neither upstream project endorses this build.

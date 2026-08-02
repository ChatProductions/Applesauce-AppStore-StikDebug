# touchHLE for iOS 0.3.0

One app, two emulator cores. HyperHLE and touchHLE now ship together, and you
choose which one runs each game. Neither is strictly better — HyperHLE runs more
games, including The Sims Medieval, while some titles only work on touchHLE 0.2.3.

## What's New

- **Both cores in one app** — HyperHLE v1.0.6 and touchHLE 0.2.3.
- **A core per game** — touch and hold a game in your library to choose its core.
  Settings sets the default for everything else.
- **JIT is checked before a game starts.** With JIT off the app used to close the
  instant a game launched, which looked like the game crashing. Now it says so.

## Upgrading

Replaces the HyperHLE 0.2.0 app in place. The separate touchHLE 0.1.0 app is no
longer needed and can be deleted once this is installed.

## Install

Sideload `touchHLE-HyperHLE-iOS-unsigned.ipa` with AltStore Classic or similar,
then enable JIT with [StikDebug](https://github.com/StephenDev0/StikDebug) +
LocalDevVPN. JIT must be enabled again every time the app starts as a new process.

Games are not included — bring your own decrypted 32-bit IPA.

Tested on an iPhone 16 Pro running iOS 27 beta 4 (24A5390f). Not an official
release of touchHLE or of HyperHLE, and neither project endorses it.

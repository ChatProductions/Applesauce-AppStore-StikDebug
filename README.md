# HyperHLE iOS Experiment

This private development branch combines the experimental native iOS port with the current HyperHLE emulator core.

## Baseline

- HyperHLE trunk commit `72cb2795`.
- Native SwiftUI library and iOS integration from the touchHLE iOS port.
- Dynarmic CPU backend; JIT remains required on iOS.
- Minimum deployment target: iOS 17.4.

## Status

This branch is for local integration and device testing only. It is not an official release of either HyperHLE or touchHLE, and it must not replace the existing stable iOS branch until its current game library has passed regression testing.

No games, signing credentials, provisioning profiles, pairing records, or device data belong in this repository.

## Credits

- [touchHLE](https://github.com/touchHLE/touchHLE) provides the original emulator architecture and core.
- [HyperHLE](https://github.com/HyperHLE/HyperHLE) provides the experimental compatibility work used by this integration.
- The native iOS shell is the experimental community port maintained separately from both upstream projects.

The source remains subject to the repository's MPL-2.0 licensing and existing third-party license requirements.

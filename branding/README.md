# Branding

Applesauce's own artwork. None of it is derived from touchHLE's or HyperHLE's
logos.

| File | Used for |
| --- | --- |
| `app-icon-light.png` | The app icon. Resized to 1024×1024 into `platform/ios/Assets.xcassets/AppIcon.appiconset/AppIcon.png` |
| `app-icon-dark.png` | The iOS 18+ dark-appearance variant, as `AppIcon-Dark.png` in the same asset set |
| `banner.png` | The image at the top of the repository README |

The About screen does not carry its own copy: it reads the app icon out of the
bundle at runtime, so replacing the asset above changes it there too.

The sources are 1254×1254. `sips -z 1024 1024 <source> --out <destination>` is
what produced the asset-catalog copies.

**On the dark variant:** iOS composites dark app icons over a system-supplied
background and expects the artwork to have a transparent one. `app-icon-dark.png`
is opaque black, so it replaces that background rather than blending with it. It
renders correctly; it just is not what Apple's guidance describes. A version with
a transparent background would be the fix.

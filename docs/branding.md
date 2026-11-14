# Branding and identity

Edit the display values in `product.json`, then run `python3 scripts/sync_product.py` and rebuild. The sync command owns generated web product metadata, Tauri configuration and distribution metadata. Stable Rust/JavaScript package names remain independent of the display name. The public Product contract keeps its existing six fields.

`name`, `slug` and `description` control browser/API text, native window titles and portable/archive distribution text. `application_id` remains explicit. Native installation values additionally come from `config/native-identity.json`:

| Field | Role |
| --- | --- |
| `application_id` | Associates the stable native configuration with the public product; synchronization rejects a mismatch before changing generated files |
| `product_name` | Stable Tauri `productName`, Debian package/resource name and NSIS product/install/registry identity |
| `wix_upgrade_code` | Explicit stable WiX upgrade UUID, preserving the original CLI-inspected default without recomputing it from mutable display text |

OS package/install labels and native bundle filenames keep the stable installation name during a display-only rename. Window titles and browser labels use the new display name; portable filenames use the new slug. Changing the native installation name, application ID or upgrade UUID is an explicit migration that needs target upgrade verification. Repository renaming and signing-key continuity are separate concerns.

The current display name is the single lowercase word **inout**. The repository and distribution slug remain `inout-next`; the stable native installation name is `Inout Next` and the application ID is `io.github.zocs.inoutnext`. This is a new installation identity, independent from FileInfra's `cc.merr.fileinfra`. Native installer/package labels can therefore still show `Inout Next`; a display rename alone does not change them.

The service reports its configured product in the API. The browser consumes that value after loading so a storage/host module does not bake in a brand. Generated metadata supplies the build-time page/window title and the separate stable native bundle identity.

The app header, footer and startup presentation share a memoized product-aware dot-matrix SVG wordmark built from local 5×7 glyph geometry. ASCII uppercase/lowercase letters and digits use the matrix up to eight characters; unsupported or longer names use wrapped text. An accessible plain-name equivalent remains available. It needs no downloaded font, bitmap asset or per-dot animation. Branding is presentation; native diagnostic authority still comes from the command/window boundary.

Startup dismissal follows actual initial web/native readiness, including authentication, stopped/no-folder setup and recoverable connection errors. There is no artificial minimum loading interval or fake progress. Opacity/transform motion uses the shared CSS timing, currently 160 ms. Cleanup reads actual computed transition durations and delays, capped at one second; ready controls never depend on an animation-completion event. Initial or live reduced-motion preference ends cleanup immediately at readiness. Escape can dismiss the waiting presentation without stopping sharing; later navigation and Start/Stop do not replay it. Browser cases cover 40/160/360 ms timings, alternate wordmarks/text and responsive layouts; they do not establish native device frame times.

For example, changing the display brand to Harbor updates web/API/window text and produces `harbor-0.1.0-linux-x64-server.zip`, while native installation name remains Inout Next and application ID remains `io.github.zocs.inoutnext`. Generated-consumer checks and emitted Debian/NSIS identity comparisons verify that separation. Package/template checks are separate from installed upgrades, MSI and runtime rebranding; see [platforms](platforms.md).

After a native release build and web export, Linux can run the isolated Debian check:

```sh
python3 scripts/verify_native_identity.py \
  --binary target/release/share-desktop \
  --target x86_64-unknown-linux-gnu --bundle deb \
  --report .local/verification/native-identity-linux.json
```

Windows CI uses the corresponding `share-desktop.exe`/target and `--bundle nsis` to compare emitted installer templates. These fixtures repackage an existing executable and verify package/template identity, unchanged source/export, window-title configuration and explicit WiX UUID invariance. They do not compile a renamed native application, install an upgrade or verify an MSI installer.

`share-server` and `share-desktop`, the `files`/`uploads`/`manage` module IDs and reserved staging namespace are stable implementation identifiers. They are not derived from displayed branding and do not need renaming to change the product. Runtime CLI branding can use `--product-file`; the native shell's metadata is synchronized and rebuilt. CI artifact labels and portable filenames consume the product slug.

The neutral transfer icon is generated by `scripts/create_icons.py`; [icon provenance](../apps/desktop/src-tauri/icons/README.md) documents its reproducible PNG/ICO/ICNS origin. Android signing identities and keys have not been introduced in this milestone.

## Store Identity and Signing Preparation

The [dated store research](distribution/store-readiness.md) proposes retaining `io.github.zocs.inoutnext` as the canonical mobile/store candidate; the existing desktop identifier does not prove store registration or namespace availability. Verify ownership/availability before first submission. Android application ID and Apple Bundle ID/Team relationships remain stable across display renames. App Store display names and listing text can change separately after an appropriate rebuild/listing update.

Android `versionCode` and Apple build numbers must be explicit, monotonic platform/distribution values, allowing more than one preview under the same display version and any intentional ABI/channel variants. Do not derive signing keys, store identities or build codes solely from the mutable slug. Preserve the six-field public Product contract.

A keystore alias selects the key entry inside a keystore; it does not set the application's name. Changing that alias cannot fix an incompatible signing certificate or rename an installed app. Keep aliases/key paths/passwords in explicit signing configuration and preserve signer fingerprints; no brand synchronization rewrites them. Play app-signing keys and upload keys have different roles. Apple signing/API keys and profiles are likewise independent of display branding and Git. [Signing and application identity](distribution/signing.md) explains reuse of FileInfra's certificate with different package IDs, signature-level trust and cross-channel updates.

F-Droid normally signs with its own key. Attempt reproducible upstream-signed APK verification early if same-ID cross-channel updates are intended, and evaluate a developer-held application signing key with a separate Play upload key before enrollment. If channels use different certificates, define identity/upgrade/data-migration consequences before first distribution; no automatic alias/product rename resolves that difference. No keys, store accounts or registrations are created by the current preparation.

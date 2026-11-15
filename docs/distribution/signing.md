# Signing and application identity

Checked against the linked official guidance on 2026-11-15. The repository is `inout-next`; the application's display name is **inout**. Display names, installation IDs and signing identities have separate roles.

The CLI's `--key-file` and native access-key generator provide HTTP authentication tokens. They are separate from application-signing keys and certificates.

## Reusing FileInfra's Android certificate

Different Android package IDs can use the same signing certificate. FileInfra uses `cc.merr.fileinfra`; the candidate identity for this project is `io.github.zocs.inoutnext`. With these different IDs, the applications install independently, keep separate application data and do not replace each other. Reusing the certificate does not migrate FileInfra data or make inout an update to FileInfra.

[Android signing guidance](https://developer.android.com/studio/publish/app-signing#considerations) allows certificate reuse across related applications. Updates still need the same package identity, an acceptable signing certificate or supported rotation lineage, and an appropriate higher version code. A shared certificate can also establish trust for [signature-level permissions](https://developer.android.com/guide/topics/permissions/overview#signature_permissions); review any exported components and such permissions explicitly. It does not automatically grant access to another application's private files. Sharing a private key also makes both applications depend on that key's protection and recovery.

FileInfra's Gradle configuration selects the `KEY_ALIAS` environment value, falling back to its historical `inout` entry. A **keystore alias is a key lookup name**, rather than an app display name or package ID. Changing a fallback from `dufshub` to `inout` is appropriate only when it selects the intended existing entry; it neither renames the app nor creates a new certificate. This project's branding generator never changes signing aliases or keys.

Before reusing FileInfra's key for a production candidate, verify the certificate fingerprints of the actual published FileInfra APK, the selected existing key entry and the new APK. Record key ownership, channel choices and rotation/backup arrangements privately. No FileInfra key was imported for the Android preview, and production/channel signing continuity remains unverified.

## Direct Android preview

The [foreground APK preview](android-preview.md) uses the independent `io.github.zocs.inoutnext.preview` package, explicit `versionCode=100001` and a retained private RSA3072 preview certificate. Its certificate SHA-256 fingerprint is:

```text
12125fbf1e85dff4cecaab432fc4cef02ba4eb37e5e36f26f6625df94842e5a2
```

The signature is verified on the final APK, separately from its SHA-256 download checksum, manifest/resource inspection and native alignment checks. The preview installs separately from FileInfra and future production `io.github.zocs.inoutnext`; Android does not automatically migrate their private data. Subsequent preview updates must retain this package/certificate and advance the explicit version code. The preview certificate does not enroll an app in Play/F-Droid or establish installed-upgrade, production-signing or store acceptance.

CI produces an unsigned single-ABI APK. Direct preview signing occurs after that artifact and its source/export are accepted; private key material and passwords are not build inputs in the public repository. A preview key is separate from an HTTP access key and from a future Play upload key.

## Channels and platforms

| Platform or channel | Reuse relationship | Identity that must remain explicit |
| --- | --- | --- |
| Android direct APK | One developer certificate may sign multiple package IDs | Each package ID, version code and certificate/rotation lineage |
| Google Play | The upload key authenticates an AAB upload; Play signs installed APKs with the app-signing key | The actual Play app-signing certificate, separate from the upload certificate |
| F-Droid | Default repository-produced APKs use F-Droid's application key; verified reproducible upstream APKs can retain the upstream signature | Same-ID cross-channel updates require compatible application signatures |
| Apple platforms | A developer team's appropriate distribution certificate may sign multiple applications | Each Bundle ID, team, matching provisioning profile and entitlements; direct distribution and App Store channels differ |
| Windows | An appropriate Authenticode publisher certificate can sign multiple products | Package/install/upgrade IDs determine coexistence; signatures authenticate the publisher |
| Linux | Debian/repository or update metadata signing belongs to its distribution channel | Package identity and channel trust; an Android keystore is not a Linux or Apple signing identity |

If the Android application should update across Play, direct downloads and F-Droid under one ID, decide the app-signing key before enrollment. [Android's Play App Signing guidance](https://developer.android.com/studio/publish/app-signing#app-signing-google-play) explains supplying an existing developer-held key when the same key is needed across stores. A matching upload-key fingerprint alone does not prove that users receive matching signatures.

[F-Droid reproducible builds](https://f-droid.org/en/docs/Reproducible_Builds/) verify the rebuilt APK against an upstream APK and its signature. This requires actual matching build bytes; using the same alias or source commit does not establish reproducibility. F-Droid application signing is also distinct from the repository/index signing key.

[Apple certificates](https://developer.apple.com/help/account/certificates/certificates-overview/) belong to an individual or team according to their type. Every app still needs the correct identifier, profile, capabilities and distribution channel. An Android keystore cannot be used as an Apple signing certificate.

The [branding contract](../branding.md) preserves installation identity across display renames. Signing and store registration remain separate acceptance gates in [store readiness](store-readiness.md).

# TidyUp — Store submission checklist

Same route as Spaceadom: a hand-laid MSIX packed with MakeAppx, uploaded to Partner Center,
re-signed by the Store. Tick in order.

## Arpon does (needs the Partner Center login)
1. Partner Center > Apps and games > New product > MSIX or PWA app > name **TidyUp** > Reserve.
   If taken, try "TidyUp Files" or "Tidy Downloads" and tell Claude the final name.
2. Product management > Product identity: copy `Package/Identity/Name` (looks like
   `SpaceZ.TidyUp`). If it is not exactly that, paste it into `src-tauri/msix/identity.json`
   as `name` (Publisher and PublisherDisplayName are already the account's values).
3. Tell Claude to rebuild: `npm run tauri build` then `npm run msix`. Output:
   `src-tauri/target/release/bundle/msix/TidyUp_1.0.0_x64.msix`.
4. Start a submission:
   - Pricing and availability: Free, all markets, discoverable.
   - Properties: category Utilities & tools; privacy policy URL from PASTE-READY-listing.md.
   - Age ratings: answer the IARC questions (all No).
   - Packages: upload the .msix. Device family: Windows 10/11 desktop only.
   - Store listing: paste every block from PASTE-READY-listing.md; upload the six screenshots
     from `screenshots/cropped/` in the listed order with their captions.
   - Submission options: paste the certification notes.
   - Submit. Certification usually takes 1 to 3 days.

## Already done (2026-09-22)
- Manifest, identity file, build script, TaskId contract check: `npm run msix` packs and validates.
- Packaged start-with-Windows via the manifest's startupTask and `packaged.rs`.
- Privacy, security, licence, third-party notices published at github.com/nur-arpon/TidyUp.
- Six screenshots, title bar and taskbar cropped, demo folder only, no personal paths.
- v1.0.0 NSIS installer on GitHub Releases for the non-Store route.

## After certification
- Add the Store link to README.md ("A Microsoft Store listing is in progress" becomes the badge).
- Update PRIVACY.md's "Store version" line if anything about packaging changed.
- Tag the release `v1.0.0-store` on the repo with the .msix attached, for the record.

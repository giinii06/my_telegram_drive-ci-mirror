# Google Drive UI

- [x] Read Model the Domain and Prove It Works principles. One derived visible-entry list controls filters, selection and previews. Browser actions and the image binary verify the actual artifact.

1. Establish the baseline first, before any migration: a visual regression harness that screenshots the current component across its states, plus the target when matching two implementations. No baseline, no parity claim. A blocking prerequisite, not a follow-up.
2. Anti-shortcut clauses, stated and held: no harness modifications, no baseline tampering, no component restructuring to make a diff pass. If the baseline looks wrong, stop and ask, don't edit it.
3. Migrate one component at a time. Parallelize across worktrees, one owner per component (the **separate-before-serializing-shared-state** principle skill). Shared primitives migrate first as a blocking phase.
4. Verify each component against its baseline via image diff on the matching surface via the driver skill. A nonzero diff is a fail. Investigate the pixel delta. `/loop` per component until the diff is zero.
5. Run **Opening a PR** per component or per safe batch.


## Execution

- [x] Remove Codex with ChatGPT requirement from its owning global AGENTS.md.
- [x] Capture existing desktop and mobile through unchanged workspace.spec.mjs. Baselines /tmp/tdrive-web-1280.png and /tmp/tdrive-web-390.png.
- [x] Inspect official Google Drive reference; matching content and account unavailable, so no pixel-exact parity claim.
- [x] Implement shared visual tokens, application shell, menus, file list and grid in the existing embedded HTML.
- [x] Exercise navigation, search, uploads, selection, dialogs, desktop and mobile.
- [x] Independent code review; fix findings.
- [x] Docker runtime image generated and verified with the exact current embedded HTML. Compose builder failed under Docker VM 2GB; Go1.25.14 host cross-compile plus unchanged runtime stages succeeded. Service not restarted.

Throughput checkpoint: keep API and existing operation handlers; adapt the DOM, CSS and local UI state only.

Opening a PR skip: local implementation requested, no publishing requested.

## Verified outcomes

- Original workspace and gallery browser harnesses unchanged. 13 browser tests pass; 3 folder-upload checks pass.
- Go tests and vet pass for cmd/tdrive and backend packages. ESLint and git diff whitespace checks pass.
- Final independent reviewer reproduced and verified fixes; no remaining actionable findings.
- Container version and JSON commands checks pass. Image 802e0a54d86b embeds exact HTML SHA256 2d7653607cd60de98bc87050e589c3eae6586017c05a30424cd3245006cf57e7.
- Old screenshots preserved in /tmp/tdrive-ui-before. Official Drive reference extracted from animated Google Workspace blog GIF at /tmp/tdrive-drive-reference-frame.png. Matching account/content is unavailable, so no zero-diff or pixel-exact equivalence claim. Step4 cannot establish visual parity across different data.
- Design candidates /tmp/tdrive-design-a.md and /tmp/tdrive-design-b.md converged on limited DOM reorganization and preserved operation handlers. No framework replacement.
- Live Telegram operations not exercised; browser checks use mock API responses. No deployment or publication performed.

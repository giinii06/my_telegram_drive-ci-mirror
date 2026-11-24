# Parallel implementation candidates

User explicitly requested parallel implementation of all phases. Existing Web-only plan and acceptance criteria remain authoritative. Parallel candidate development is not a waiver of completion, CI or E2E gates. Use independent branches from a common reviewed base, without merging or deploying. No production account/data access.

First independent workstreams: Phase 07 backup digest verification (tdrive-backup only); Phase 08 bounded range streaming (tdrive-media only); Phase 09 WebDAV lock-header contracts (new tdrive-mount crate, root Cargo manifests). Test candidates first, implementation commits second, exact-SHA CI separately. Other phases remain in existing ledger until their code candidates exist. No agent independence is claimed; actual tool calls may run concurrently while source review is performed by one assistant. Final integration and Go deletion remain dependency-gated.

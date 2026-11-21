# TDrive 기술 문서

이 디렉터리에는 설계 및 변경 계획을 보관합니다.

## Rust + GPUI + Docker 전환

- [마이그레이션 전체 계획](migration/README.md)
- [현재 구조 및 근거](migration/current-state.md)
- [목표 아키텍처](migration/architecture.md)
- [호환성 매트릭스](migration/compatibility-matrix.md)
- [결정 기록 및 미해결 질문](migration/decision-log.md)
- [Phase 00 ~ 15 실행 문서](migration/README.md#phase-별-문서)

> **이 문서 묶음은 실행 계획이며 구현 완료 보고서가 아니다.** 이 MR에서는 기존 Go, Svelte, Docker, 모바일 패키징 코드를 변경하지 않는다. 각 Phase는 후속 MR에서 따로 구현하고 테스트한 뒤 다음 단계로 진행한다.

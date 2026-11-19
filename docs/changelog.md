# Changelog

## Unreleased

### 0.1.2

- Added an ordered implementation task list and concrete runtime choices: isolated
  PDF workers/audience host, progressive image caches, bounded recovery, native sleep
  inhibition and measured latency budgets. Dependency/build/package size constraints
  do not take precedence over fast startup and resilience. Require portable executables
  with bundled non-system dependencies and offline clean-machine tests. Choices are researched;
  desktop implementation and physical acceptance remain pending.

### 0.1.1

- Established selectable dual-screen operation on Windows/macOS/Linux as a hard
  acceptance gate and presentation speed as the primary performance goal. Prioritized
  progressive rendering, reusable source-SHA image caches and current/next previews
  with extractable notes. Added physical-display and speed validation requirements;
  cache schema and application behavior remain unimplemented.

### 0.1.0

- Prepared the standalone PDF Presenter repository with a tested navigation/blanking
  core, an explicit bootstrap executable, agent guidance and implementation plan.
- Added Jenkins validation, whole-workspace coverage and isolated three-badge
  publication, plus generated application/repository artwork. PDF rendering and
  multiscreen GUI are future work; no physical-output acceptance is claimed.

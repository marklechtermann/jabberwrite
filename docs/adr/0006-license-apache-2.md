---
status: accepted  # proposed | accepted | superseded by NNNN
date: 2026-10-29
---

# 0006: License Jabberwrite under Apache-2.0 only

## Context

The project started with the Rust convention of dual-licensing under `MIT OR Apache-2.0`. That convention exists mainly for GPLv2 compatibility of libraries. Jabberwrite is an end-user desktop application, not a library, so this does not matter here. A single license is simpler, and its main runtime dependency, sherpa-onnx, is already Apache-2.0.

## Decision

Jabberwrite is licensed under Apache-2.0 only. The license text lives in `LICENSE`; `LICENSE-MIT` and `LICENSE-APACHE` are removed. Package metadata uses `Apache-2.0`.

This supersedes the dual-license mention in ADR 0002. Earlier releases stay available under the terms they were published with.

## Consequences

- One license, no "which one applies" question.
- Explicit patent grant and contribution terms.
- Not combinable with GPLv2-only code; some companies with MIT-only policies may be unable to reuse the code. Accepted for an application.

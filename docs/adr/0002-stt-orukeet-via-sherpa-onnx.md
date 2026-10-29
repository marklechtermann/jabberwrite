---
status: accepted
date: 2026-10-29
---

# 0002: Speech-to-text: Orukeet model via sherpa-onnx

## Context

We want to use [Orukeet](https://github.com/Oruk-AI/orukeet), a 25-language fine-tune of NVIDIA Parakeet TDT 0.6B v3. It is a model, not an engine; there is no Rust crate. It ships as:

- a Python package (spawns a Python worker that loads a C library),
- GGUF weights for NVIDIA NeMo-Speech.cpp (C ABI, CPU/CUDA/Vulkan/Metal),
- ONNX INT8 weights (≈487 MB download) in the standard Parakeet TDT v3 layout, loadable by sherpa-onnx.

The model expects mono 16 kHz float32 PCM and transcribes whole recordings (batch). It does not detect the language. The weights are CC BY-SA 4.0 with NVIDIA attribution; the code is MIT. The project is young (created 2026-09).

## Decision

- Run the ONNX INT8 model through sherpa-onnx's Rust bindings, CPU only.
- Link ONNX Runtime statically where the bindings allow; otherwise bundle it as a Tauri resource.
- Do not bundle the model. On first launch, download it from Hugging Face (`oruk/orukeet`) at a pinned revision, verify SHA-256, and store it in the per-user data directory. `config.json` may override the model path.
- Load the model once at startup and keep it in memory.
- No language setting; the model handles multilingual input on its own.
- Credit Orukeet and NVIDIA Parakeet (with links and licence) in the settings dialog and README. Jabberwrite itself is dual-licensed MIT / Apache-2.0.

## Consequences

- No Python, no sidecar process; one native binary plus the ONNX Runtime library.
- The same engine can later run other sherpa-onnx models.
- About 1 GB RAM is used while idle; first launch needs a ≈487 MB download.
- No GPU acceleration in v1. If CPU latency on Windows/Linux is too slow, NeMo-Speech.cpp (GGUF, GPU backends) is the fallback, recorded in a new ADR.

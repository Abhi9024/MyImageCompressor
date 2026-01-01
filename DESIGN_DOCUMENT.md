# Medical Image Compression Application - Design Document

**Version:** 1.0
**Date:** December 2024
**Author:** AI Design Assistant

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Language Selection: Rust vs C++](#2-language-selection-rust-vs-c)
3. [Medical Image Compression Techniques Analysis](#3-medical-image-compression-techniques-analysis)
4. [Recommended Compression Strategy](#4-recommended-compression-strategy)
5. [System Architecture](#5-system-architecture)
6. [Module Design](#6-module-design)
7. [Data Flow](#7-data-flow)
8. [Regulatory Compliance](#8-regulatory-compliance)
9. [Performance Requirements](#9-performance-requirements)
10. [Future Considerations](#10-future-considerations)

---

## 1. Executive Summary

This document outlines the design for a medical image compression application targeting healthcare environments. The application will handle DICOM images (the industry standard for medical imaging) and provide both lossless and lossy compression options while maintaining diagnostic quality and regulatory compliance.

**Key Goals:**
- Achieve high compression ratios while preserving diagnostic quality
- Full DICOM standard compliance
- Memory-safe, high-performance implementation
- Support for multiple imaging modalities (CT, MRI, X-Ray, Mammography, WSI)
- Regulatory compliance (FDA, ACR guidelines)

---

## 2. Language Selection: Rust vs C++

### 2.1 Comparison Matrix

| Criteria | C++ | Rust | Winner |
|----------|-----|------|--------|
| **Performance** | Excellent | Excellent (comparable) | Tie |
| **Memory Safety** | Manual management, prone to errors | Compile-time guarantees | Rust |
| **Medical Imaging Libraries** | Mature (DCMTK, ITK, GDCM) | Growing (dicom-rs) | C++ |
| **Concurrency Safety** | Manual, error-prone | Built-in, safe by default | Rust |
| **Security** | Vulnerable to memory exploits | Memory-safe by design | Rust |
| **Learning Curve** | Moderate | Steep | C++ |
| **Build System** | Complex (CMake, etc.) | Excellent (Cargo) | Rust |
| **WebAssembly Support** | Limited | First-class | Rust |
| **Long-term Maintenance** | Higher technical debt | Lower maintenance burden | Rust |

### 2.2 Medical Imaging Specific Considerations

**C++ Advantages:**
- DCMTK: The most comprehensive DICOM toolkit (mature, FDA-validated in many products)
- ITK/VTK: Industry-standard for image processing
- OpenJPEG: Reference JPEG 2000 implementation
- Vast ecosystem of validated medical imaging tools

**Rust Advantages:**
- **dicom-rs**: Active open-source DICOM implementation in pure Rust
- Memory safety critical for medical systems (exploited vulnerabilities could compromise medical data)
- Zero-cost abstractions for performance-critical operations
- Excellent FFI support to leverage existing C/C++ libraries when needed
- Growing adoption in medical imaging informatics (data processing pipelines, voxel renderers)

### 2.3 Decision: **Rust**

**Rationale:**

1. **Patient Safety & Security**: In medical imaging systems, memory vulnerabilities can lead to:
   - System downtime during critical diagnoses
   - Compromised medical data (HIPAA violations)
   - Inaccurate information display
   - Rust eliminates entire classes of memory-related bugs at compile time

2. **Performance Parity**: Rust matches C++ performance for CPU-bound image processing operations

3. **Modern Concurrency**: Medical imaging increasingly requires parallel processing (large datasets, real-time requirements). Rust's ownership model prevents data races by design

4. **FFI Capability**: Can interface with existing C libraries (OpenJPEG, CharLS) when mature Rust alternatives don't exist

5. **Future-Proofing**: Growing ecosystem, WebAssembly support for web-based PACS viewers, active community development of dicom-rs

6. **Reduced Technical Debt**: Compiler-enforced safety reduces long-term maintenance costs critical for medical software lifecycle

---

## 3. Medical Image Compression Techniques Analysis

### 3.1 Overview of DICOM-Supported Compression Standards

The American College of Radiology (ACR) mandates that only DICOM-standard algorithms be used: JPEG, JPEG-LS, JPEG-2000, or MPEG.

### 3.2 Detailed Analysis

---

#### 3.2.1 JPEG Lossless (Process 14)

**Description:** Predictive coding model using spatial prediction. Original DICOM lossless standard.

**Transfer Syntax UID:** 1.2.840.10008.1.2.4.70

| Pros | Cons |
|------|------|
| Universal support in all DICOM viewers | Lower compression ratio (~2:1 to 3:1) |
| Simple implementation | No progressive decoding |
| Fast encoding/decoding | No ROI (Region of Interest) support |
| Well-validated in clinical use | Single-resolution output only |
| Low computational overhead | Outdated compared to modern codecs |

**Compression Ratio:** ~2.5:1 (typical)

**Best For:** Legacy system compatibility, simple archival

---

#### 3.2.2 JPEG-LS (Lossless/Near-Lossless)

**Description:** ITU-T T.87 standard using LOCO-I algorithm (Low Complexity Lossless Compression for Images).

**Transfer Syntax UID:** 1.2.840.10008.1.2.4.80 (lossless), 1.2.840.10008.1.2.4.81 (near-lossless)

| Pros | Cons |
|------|------|
| Excellent compression ratio (~3.8:1) | No progressive decoding |
| Very fast encoding/decoding | No multi-resolution support |
| Low memory requirements | Limited ROI capability |
| Near-lossless mode available | Less widespread viewer support than JPEG 2000 |
| Simple, efficient algorithm | No tiling support for large images |

**Compression Ratio:** ~3.8:1 (lossless), higher with near-lossless

**Best For:** High-throughput environments, edge devices, real-time applications

---

#### 3.2.3 JPEG 2000 (Part 1)

**Description:** Wavelet-based compression using Discrete Wavelet Transform (DWT). The current gold standard for medical imaging.

**Transfer Syntax UIDs:**
- 1.2.840.10008.1.2.4.90 (Lossless)
- 1.2.840.10008.1.2.4.91 (Lossy)

| Pros | Cons |
|------|------|
| Excellent compression (lossless ~3.8:1, lossy 10:1+) | Computationally expensive |
| Progressive decoding (resolution/quality layers) | Complex implementation |
| ROI coding support | Slower than JPEG-LS |
| Tiling for large images (WSI) | Higher memory requirements |
| Superior lossy quality vs. JPEG | Patent concerns (historically) |
| Multi-resolution representation | |
| Error resilience | |
| Both lossless and lossy in same codestream | |

**Compression Ratio:** ~3.8:1 (lossless), 10:1 to 50:1 (lossy, diagnostically acceptable)

**Best For:** PACS archival, whole-slide imaging, teleradiology, general medical imaging

---

#### 3.2.4 HTJ2K (High-Throughput JPEG 2000)

**Description:** ISO/IEC 15444-15. Block-based entropy coding replacing arithmetic coding for dramatically faster processing while maintaining JPEG 2000 features.

**Transfer Syntax UIDs:**
- 1.2.840.10008.1.2.4.201 (Lossless)
- 1.2.840.10008.1.2.4.202 (Lossless RPCL - progressive)
- 1.2.840.10008.1.2.4.203 (Any)

| Pros | Cons |
|------|------|
| 10-30x faster than JPEG 2000 | Newer, less widespread support |
| Maintains all J2K features | Slightly lower compression ratio |
| Progressive decoding | Viewer adoption still growing |
| Backward compatible with J2K ecosystem | |
| Ideal for real-time applications | |
| GPU-friendly architecture | |

**Compression Ratio:** ~3.5:1 (lossless), comparable to J2K lossy

**Best For:** Real-time viewing, streaming, modern PACS systems, cloud-based imaging

---

#### 3.2.5 JPEG-XL

**Description:** Next-generation codec (ISO/IEC 18181). Recently added to DICOM standard.

**Transfer Syntax UIDs:**
- 1.2.840.10008.1.2.4.110 (Lossless)
- 1.2.840.10008.1.2.4.111 (JPEG Recompression)
- 1.2.840.10008.1.2.4.112 (Any)

| Pros | Cons |
|------|------|
| Superior compression ratios | Very new to medical imaging |
| Lossless JPEG recompression | Limited DICOM viewer support |
| Progressive decoding | Ecosystem still maturing |
| HDR support | Regulatory validation pending |
| Responsive/adaptive decoding | |
| Fast encoding (VarDCT mode) | |

**Compression Ratio:** ~4:1+ (lossless), superior lossy performance

**Best For:** Future consideration, archival migration, research environments

---

#### 3.2.6 RLE (Run-Length Encoding)

**Description:** Simple lossless compression based on repeating value sequences.

**Transfer Syntax UID:** 1.2.840.10008.1.2.5

| Pros | Cons |
|------|------|
| Extremely simple | Poor compression for most medical images |
| Very fast | Only efficient for specific image types |
| Universal support | Typically < 2:1 ratio |

**Best For:** Binary masks, segmentation overlays, specific modalities with uniform regions

---

### 3.3 Compression Technique Comparison Summary

| Technique | Lossless Ratio | Speed (Encode) | Speed (Decode) | Progressive | ROI | DICOM Support | Maturity |
|-----------|----------------|----------------|----------------|-------------|-----|---------------|----------|
| JPEG Lossless | 2.5:1 | Fast | Fast | No | No | Universal | High |
| JPEG-LS | 3.8:1 | Very Fast | Very Fast | No | Limited | Good | High |
| JPEG 2000 | 3.8:1 | Slow | Moderate | Yes | Yes | Universal | High |
| HTJ2K | 3.5:1 | Fast | Very Fast | Yes | Yes | Growing | Medium |
| JPEG-XL | 4.0:1+ | Moderate | Fast | Yes | Yes | New | Low |
| RLE | 1.5:1 | Very Fast | Very Fast | No | No | Universal | High |

---

## 4. Recommended Compression Strategy

### 4.1 Primary Recommendation: **JPEG 2000 with HTJ2K Acceleration**

**Selected Approach:** Implement JPEG 2000 as the primary codec with HTJ2K as the preferred encoding for new data, while maintaining full JPEG 2000 Part 1 compatibility.

### 4.2 Justification

1. **Industry Standard**: JPEG 2000 is the established gold standard for medical imaging compression
   - Supported by virtually all modern PACS systems
   - Validated in clinical use for decades
   - ACR-compliant

2. **Feature Completeness**:
   - Progressive decoding essential for teleradiology (view while downloading)
   - Multi-resolution representation ideal for large images (CT volumes, WSI)
   - ROI coding for diagnostic focus areas
   - Tiling support for gigapixel whole-slide images

3. **Flexibility**:
   - Single codestream supports both lossless and lossy decoding
   - Quality layers allow "visually lossless" intermediate options
   - Scalable from mobile viewing to full diagnostic quality

4. **HTJ2K Benefits**:
   - Addresses JPEG 2000's primary weakness (speed)
   - 10-30x faster encoding/decoding
   - Backward compatible with existing J2K infrastructure
   - Future-proof (growing DICOM adoption)

5. **Regulatory Safety**:
   - FDA and ACR recognized
   - Lossless mode for mammography and primary archival
   - Well-defined quality metrics for lossy use

### 4.3 Secondary Codec: **JPEG-LS**

Include JPEG-LS as a secondary option for:
- Edge devices with limited compute
- Real-time acquisition systems
- Scenarios where progressive decoding isn't needed
- Maximum encoding speed requirements

### 4.4 Modality-Specific Recommendations

| Modality | Primary Codec | Mode | Rationale |
|----------|---------------|------|-----------|
| **Mammography** | JPEG 2000 | Lossless Only | FDA requirement |
| **CT** | HTJ2K/JPEG 2000 | Lossless or Lossy | Volume data, progressive viewing |
| **MRI** | HTJ2K/JPEG 2000 | Lossless or Lossy | Multi-sequence, progressive |
| **X-Ray** | HTJ2K/JPEG 2000 | Lossless preferred | General radiography |
| **Ultrasound** | JPEG 2000 | Lossy acceptable | Real-time, cine loops |
| **Whole Slide Imaging** | JPEG 2000 | Lossy acceptable | Gigapixel, tiling critical |
| **Nuclear Medicine** | JPEG-LS | Lossless | Lower resolution, fast |

---

## 5. System Architecture

### 5.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Medical Image Compression System                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────────┐ │
│  │   DICOM     │    │   Codec     │    │  Pipeline   │    │  Output  │ │
│  │   Parser    │───▶│   Engine    │───▶│  Manager    │───▶│  Writer  │ │
│  └─────────────┘    └─────────────┘    └─────────────┘    └──────────┘ │
│         │                  │                  │                  │      │
│         ▼                  ▼                  ▼                  ▼      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     Configuration Layer                          │   │
│  │  • Modality Rules  • Quality Profiles  • Regulatory Constraints │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     Core Libraries (FFI)                         │   │
│  │  • OpenJPEG (J2K)  • OpenHTJ2K  • CharLS (JPEG-LS)              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Component Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         Application Layer                         │
├──────────────────────────────────────────────────────────────────┤
│  CLI Interface  │  Library API (Rust)  │  C-API (FFI exports)   │
└────────┬────────┴──────────┬───────────┴────────────┬────────────┘
         │                   │                        │
         ▼                   ▼                        ▼
┌──────────────────────────────────────────────────────────────────┐
│                         Service Layer                             │
├──────────────────────────────────────────────────────────────────┤
│  CompressionService  │  ValidationService  │  MetadataService    │
└──────────┬───────────┴─────────┬───────────┴──────────┬──────────┘
           │                     │                      │
           ▼                     ▼                      ▼
┌──────────────────────────────────────────────────────────────────┐
│                          Core Layer                               │
├──────────────────────────────────────────────────────────────────┤
│  DicomParser  │  ImageProcessor  │  CodecManager  │  IOHandler   │
└───────┬───────┴────────┬─────────┴───────┬────────┴──────┬───────┘
        │                │                 │               │
        ▼                ▼                 ▼               ▼
┌──────────────────────────────────────────────────────────────────┐
│                        Codec Layer                                │
├──────────────────────────────────────────────────────────────────┤
│  J2KCodec  │  HTJ2KCodec  │  JPEGLSCodec  │  RLECodec           │
└──────────────────────────────────────────────────────────────────┘
```

---

## 6. Module Design

### 6.1 Core Modules

#### 6.1.1 `dicom_parser`
- Parse DICOM files (Part 10 format)
- Extract pixel data and metadata
- Handle encapsulated pixel data
- Support transfer syntax negotiation

#### 6.1.2 `codec_engine`
```rust
pub trait Codec {
    fn encode(&self, image: &ImageData, config: &CodecConfig) -> Result<EncodedData>;
    fn decode(&self, data: &[u8], config: &CodecConfig) -> Result<ImageData>;
    fn supports_lossless(&self) -> bool;
    fn supports_progressive(&self) -> bool;
    fn transfer_syntax_uid(&self) -> &str;
}
```

#### 6.1.3 `pipeline_manager`
- Orchestrate compression workflows
- Manage parallel processing (Rayon)
- Handle batch operations
- Progress reporting

#### 6.1.4 `config_manager`
- Modality-specific compression rules
- Quality profiles (diagnostic, viewing, thumbnail)
- Regulatory constraints enforcement
- User-defined presets

### 6.2 Codec Implementations

#### 6.2.1 `j2k_codec`
- FFI bindings to OpenJPEG
- Lossless and lossy modes
- Quality layer configuration
- Tile size optimization
- ROI encoding support

#### 6.2.2 `htj2k_codec`
- FFI bindings to OpenHTJ2K
- High-throughput encoding
- Progressive decoding support
- GPU acceleration (optional)

#### 6.2.3 `jpegls_codec`
- FFI bindings to CharLS
- Lossless and near-lossless modes
- Interleave mode handling

### 6.3 Support Modules

#### 6.3.1 `validation`
- Input image validation
- Compression ratio verification
- Quality metrics (PSNR, SSIM)
- DICOM conformance checking

#### 6.3.2 `metadata`
- DICOM tag preservation
- Transfer syntax updates
- Compression metadata injection
- Audit trail support

---

## 7. Data Flow

### 7.1 Compression Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Compression Data Flow                            │
└─────────────────────────────────────────────────────────────────────────┘

  Input DICOM          Parse &              Select           Compress
  File                 Validate             Codec            Image
      │                    │                   │                │
      ▼                    ▼                   ▼                ▼
 ┌─────────┐        ┌───────────┐       ┌──────────┐     ┌──────────┐
 │ .dcm    │───────▶│ Extract   │──────▶│ Modality │────▶│ Encode   │
 │ file    │        │ PixelData │       │ Rules    │     │ (J2K/LS) │
 └─────────┘        │ Metadata  │       │ Config   │     └────┬─────┘
                    └───────────┘       └──────────┘          │
                                                              ▼
                                                        ┌──────────┐
  Output DICOM         Update              Validate     │ Encoded  │
  File                 Headers             Output       │ Stream   │
      ▲                    ▲                   ▲        └────┬─────┘
      │                    │                   │             │
 ┌─────────┐        ┌───────────┐       ┌──────────┐        │
 │ .dcm    │◀───────│ Write     │◀──────│ Verify   │◀───────┘
 │ file    │        │ DICOM     │       │ Quality  │
 └─────────┘        └───────────┘       └──────────┘
```

### 7.2 Batch Processing Flow

```
┌────────────┐     ┌─────────────┐     ┌─────────────┐     ┌────────────┐
│  Input     │     │   Work      │     │  Parallel   │     │  Output    │
│  Queue     │────▶│   Splitter  │────▶│  Workers    │────▶│  Collector │
└────────────┘     └─────────────┘     └─────────────┘     └────────────┘
                                              │
                                              ▼
                                       ┌─────────────┐
                                       │  Progress   │
                                       │  Reporter   │
                                       └─────────────┘
```

---

## 8. Regulatory Compliance

### 8.1 FDA Requirements

- **Mammography**: Lossless compression ONLY for archival (FDA guidance)
- No lossy compression for primary diagnosis storage
- Audit trails for compression operations
- Reversibility verification for lossless claims

### 8.2 ACR Technical Standards

- Use only DICOM-defined compression algorithms
- Document compression ratios in DICOM headers
- Maintain original pixel data for legal/regulatory retention
- Quality assurance protocols for lossy compression

### 8.3 Implementation Safeguards

```rust
pub struct CompressionPolicy {
    modality: Modality,
    max_lossy_ratio: Option<f32>,
    require_lossless: bool,
    require_audit: bool,
    quality_validation: QualityThreshold,
}

// Example: Mammography policy
const MAMMOGRAPHY_POLICY: CompressionPolicy = CompressionPolicy {
    modality: Modality::MG,
    max_lossy_ratio: None,  // Lossy not allowed
    require_lossless: true,
    require_audit: true,
    quality_validation: QualityThreshold::BitExact,
};
```

---

## 9. Performance Requirements

### 9.1 Target Metrics

| Metric | Target | Rationale |
|--------|--------|-----------|
| **Throughput (Lossless)** | > 100 MB/s | Batch archival needs |
| **Throughput (HTJ2K)** | > 200 MB/s | Real-time viewing |
| **Latency (Single Image)** | < 500ms | Interactive use |
| **Memory Usage** | < 2x image size | Resource-constrained systems |
| **CPU Utilization** | Scale to all cores | Parallel processing |

### 9.2 Optimization Strategies

1. **Parallel Processing**: Use Rayon for data-parallel compression
2. **Memory Mapping**: Memory-mapped I/O for large files
3. **Streaming**: Process tiles/chunks without full image in memory
4. **SIMD**: Leverage SIMD instructions via codec libraries
5. **GPU Acceleration**: Optional OpenCL/CUDA for HTJ2K (future)

---

## 10. Future Considerations

### 10.1 Roadmap

**Phase 1 (MVP):**
- JPEG 2000 lossless/lossy
- JPEG-LS lossless
- Basic CLI interface
- Single-file processing

**Phase 2:**
- HTJ2K support
- Batch processing
- Progress API
- Quality metrics

**Phase 3:**
- Library API with C-bindings
- WebAssembly build
- GPU acceleration
- JPEG-XL experimental support

### 10.2 Technology Watch

- **JPEG-XL**: Monitor DICOM viewer adoption for potential future inclusion
- **AI-based compression**: Emerging neural compression codecs
- **Cloud integration**: Streaming compression for cloud PACS

---

## Appendix A: References

1. [The Current Role of Image Compression Standards in Medical Imaging - PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC8525863/)
2. [DICOM Standard - JPEG 2000 Image Compression](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_a.4.4.html)
3. [JPEG 2000 in DICOM - MiNNOVAA](https://minnovaa.com/jpeg-2000/)
4. [Why Rust in Medical Imaging - BMD Software](https://www.bmd-software.com/news/why-rust-in-medical-imaging-a-reflection-on-modern-technologies-for-next-generation-systems/)
5. [Rust for Computer Vision - Ataiva](https://ataiva.com/rust-computer-vision-ecosystem/)
6. [DICOM Standard Supplement 232 - JPEG-XL](https://www.dicomstandard.org/News-dir/ftsup/docs/sups/sup232.pdf)
7. [Optimized JPEG 2000 Compression for WSI - PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC5989536/)
8. [Should I Compress DICOM Images? - Purview](https://www.purview.net/blog/compress-dicom-images)

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| **DICOM** | Digital Imaging and Communications in Medicine |
| **PACS** | Picture Archiving and Communication System |
| **WSI** | Whole Slide Imaging |
| **HTJ2K** | High-Throughput JPEG 2000 |
| **ROI** | Region of Interest |
| **Transfer Syntax** | DICOM encoding format identifier |
| **Lossless** | Compression with exact reconstruction |
| **Lossy** | Compression with controlled quality loss |
| **ACR** | American College of Radiology |
| **FDA** | Food and Drug Administration |

---

*Document End*

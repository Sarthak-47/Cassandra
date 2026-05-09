"""Universal compression with ML-based content detection.

This module provides intelligent, automatic compression that:
1. Detects content type using ML (Magika)
2. Preserves structure (keys, signatures, templates)
3. Compresses content with Kompress
4. Enables retrieval via CCR

Quick Start:
    # One-liner for simple use
    from cassandra.compression import compress
    result = compress(content)

    # Or with configuration
    from cassandra.compression import UniversalCompressor, UniversalCompressorConfig

    config = UniversalCompressorConfig(compression_ratio_target=0.5)
    compressor = UniversalCompressor(config=config)
    result = compressor.compress(content)
"""

from cassandra.compression.detector import ContentType, MagikaDetector
from cassandra.compression.masks import StructureMask
from cassandra.compression.universal import (
    CompressionResult,
    UniversalCompressor,
    UniversalCompressorConfig,
    compress,
)

__all__ = [
    # Simple API
    "compress",
    # Full API
    "UniversalCompressor",
    "UniversalCompressorConfig",
    "CompressionResult",
    # Advanced
    "MagikaDetector",
    "ContentType",
    "StructureMask",
]

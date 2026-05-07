"""
Cassandra - The Context Optimization Layer for LLM Applications.

Cut your LLM costs by 50-90% without losing accuracy.

Cassandra wraps LLM clients to provide:
- Smart compression of tool outputs (keeps errors, anomalies, relevant items)
- Cache-aligned prefix optimization for better provider cache hits
- Rolling window token management for long conversations
- Full streaming support with zero accuracy loss

Quick Start:

    from cassandra import CassandraClient, OpenAIProvider
    from openai import OpenAI

    # Wrap your existing client
    client = CassandraClient(
        original_client=OpenAI(),
        provider=OpenAIProvider(),
        default_mode="optimize",
    )

    # Use exactly like the original client
    response = client.chat.completions.create(
        model="gpt-4o",
        messages=[
            {"role": "user", "content": "Hello!"},
        ],
    )

    # Check savings
    stats = client.get_stats()
    print(f"Tokens saved: {stats['session']['tokens_saved_total']}")

Verify It's Working:

    # Validate configuration
    result = client.validate_setup()
    if not result["valid"]:
        print("Issues:", result)

    # Enable logging to see what's happening
    import logging
    logging.basicConfig(level=logging.INFO)
    # INFO:cassandra.transforms.pipeline:Pipeline complete: 45000 -> 4500 tokens

Simulate Before Sending:

    plan = client.chat.completions.simulate(
        model="gpt-4o",
        messages=large_messages,
    )
    print(f"Would save {plan.tokens_saved} tokens")
    print(f"Transforms: {plan.transforms}")

Error Handling:

    from cassandra import CassandraError, ConfigurationError, ProviderError

    try:
        response = client.chat.completions.create(...)
    except ConfigurationError as e:
        print(f"Config issue: {e.details}")
    except CassandraError as e:
        print(f"Cassandra error: {e}")

For more examples, see https://github.com/cassandra-sdk/cassandra/tree/main/examples
"""

from __future__ import annotations

from importlib import import_module
from typing import Any

from ._ort import ensure_ort_dylib_pinned
from ._version import __version__  # noqa: F401

# Must run before anything can import `cassandra._core`: on Windows the
# Rust core resolves onnxruntime.dll at runtime (ort load-dynamic), and
# the bare DLL search lands on the Windows ML System32 build, which
# deadlocks ort session init (Win11 24H2+). Windows-gated, idempotent,
# ~microseconds. See `cassandra/_ort.py` for the full story.
ensure_ort_dylib_pinned()

from .compress import CompressConfig, CompressResult, compress, compress_spreadsheet  # noqa: E402

# Keep a real callable bound for the one-function compression API so
# `from cassandra import compress` is never shadowed by the submodule object.

__all__ = [
    # Main client
    "CassandraClient",
    # Providers
    "Provider",
    "TokenCounter",
    "OpenAIProvider",
    "AnthropicProvider",
    # Exceptions
    "CassandraError",
    "ConfigurationError",
    "ProviderError",
    "StorageError",
    "CompressionError",
    "TokenizationError",
    "CacheError",
    "ValidationError",
    "TransformError",
    # Config
    "CassandraConfig",
    "CassandraMode",
    "SmartCrusherConfig",
    "CacheAlignerConfig",
    "CacheOptimizerConfig",
    "RelevanceScorerConfig",
    # Data models
    "Block",
    "CachePrefixMetrics",
    "DiffArtifact",
    "RequestMetrics",
    "SimulationResult",
    "TransformDiff",
    "TransformResult",
    "WasteSignals",
    # Transforms
    "SmartCrusher",
    "CacheAligner",
    "TransformPipeline",
    # Cache optimizers
    "BaseCacheOptimizer",
    "CacheConfig",
    "CacheMetrics",
    "CacheResult",
    "CacheStrategy",
    "OptimizationContext",
    "CacheOptimizerRegistry",
    "AnthropicCacheOptimizer",
    "OpenAICacheOptimizer",
    "GoogleCacheOptimizer",
    "SemanticCache",
    "SemanticCacheLayer",
    # Relevance scoring - BM25 always available, embeddings require sentence-transformers
    "RelevanceScore",
    "RelevanceScorer",
    "BM25Scorer",
    "EmbeddingScorer",
    "HybridScorer",
    "create_scorer",
    "embedding_available",
    # Utilities
    "Tokenizer",
    "count_tokens_text",
    "count_tokens_messages",
    "generate_report",
    # Observability
    "CassandraOtelMetrics",
    "CassandraTracer",
    "LangfuseTracingConfig",
    "OTelMetricsConfig",
    "configure_otel_metrics",
    "configure_langfuse_tracing",
    "get_cassandra_tracer",
    "get_langfuse_tracing_status",
    "get_otel_metrics",
    "get_otel_metrics_status",
    "reset_cassandra_tracing",
    "reset_otel_metrics",
    # Memory - optional hierarchical memory system
    "with_memory",  # Main user-facing API
    "Memory",
    "ScopeLevel",
    "HierarchicalMemory",
    "MemoryConfig",
    "EmbedderBackend",
    # One-function compression API
    "compress",
    "compress_spreadsheet",
    "CompressConfig",
    "CompressResult",
    # Hooks
    "CompressionHooks",
    "CompressContext",
    "CompressEvent",
    # Canonical pipeline
    "PipelineStage",
    "PipelineEvent",
    "PipelineExtensionManager",
    "CANONICAL_PIPELINE_STAGES",
    # Shared context for multi-agent workflows
    "SharedContext",
]

# Keep package-level imports lightweight so `import cassandra` does not eagerly
# load provider SDKs, ML stacks, or optional proxy/runtime integrations.
_LAZY_EXPORTS: dict[str, tuple[str, str]] = {
    # Main client
    "CassandraClient": ("cassandra.client", "CassandraClient"),
    # Providers
    "Provider": ("cassandra.providers", "Provider"),
    "TokenCounter": ("cassandra.providers", "TokenCounter"),
    "OpenAIProvider": ("cassandra.providers", "OpenAIProvider"),
    "AnthropicProvider": ("cassandra.providers", "AnthropicProvider"),
    # Exceptions
    "CassandraError": ("cassandra.exceptions", "CassandraError"),
    "ConfigurationError": ("cassandra.exceptions", "ConfigurationError"),
    "ProviderError": ("cassandra.exceptions", "ProviderError"),
    "StorageError": ("cassandra.exceptions", "StorageError"),
    "CompressionError": ("cassandra.exceptions", "CompressionError"),
    "TokenizationError": ("cassandra.exceptions", "TokenizationError"),
    "CacheError": ("cassandra.exceptions", "CacheError"),
    "ValidationError": ("cassandra.exceptions", "ValidationError"),
    "TransformError": ("cassandra.exceptions", "TransformError"),
    # Config
    "CassandraConfig": ("cassandra.config", "CassandraConfig"),
    "CassandraMode": ("cassandra.config", "CassandraMode"),
    "SmartCrusherConfig": ("cassandra.config", "SmartCrusherConfig"),
    "CacheAlignerConfig": ("cassandra.config", "CacheAlignerConfig"),
    "CacheOptimizerConfig": ("cassandra.config", "CacheOptimizerConfig"),
    "RelevanceScorerConfig": ("cassandra.config", "RelevanceScorerConfig"),
    # Data models
    "Block": ("cassandra.config", "Block"),
    "CachePrefixMetrics": ("cassandra.config", "CachePrefixMetrics"),
    "DiffArtifact": ("cassandra.config", "DiffArtifact"),
    "RequestMetrics": ("cassandra.config", "RequestMetrics"),
    "SimulationResult": ("cassandra.config", "SimulationResult"),
    "TransformDiff": ("cassandra.config", "TransformDiff"),
    "TransformResult": ("cassandra.config", "TransformResult"),
    "WasteSignals": ("cassandra.config", "WasteSignals"),
    # Transforms
    "SmartCrusher": ("cassandra.transforms", "SmartCrusher"),
    "CacheAligner": ("cassandra.transforms", "CacheAligner"),
    "TransformPipeline": ("cassandra.transforms", "TransformPipeline"),
    # Cache optimizers
    "BaseCacheOptimizer": ("cassandra.cache", "BaseCacheOptimizer"),
    "CacheConfig": ("cassandra.cache", "CacheConfig"),
    "CacheMetrics": ("cassandra.cache", "CacheMetrics"),
    "CacheResult": ("cassandra.cache", "CacheResult"),
    "CacheStrategy": ("cassandra.cache", "CacheStrategy"),
    "OptimizationContext": ("cassandra.cache", "OptimizationContext"),
    "CacheOptimizerRegistry": ("cassandra.cache", "CacheOptimizerRegistry"),
    "AnthropicCacheOptimizer": ("cassandra.cache", "AnthropicCacheOptimizer"),
    "OpenAICacheOptimizer": ("cassandra.cache", "OpenAICacheOptimizer"),
    "GoogleCacheOptimizer": ("cassandra.cache", "GoogleCacheOptimizer"),
    "SemanticCache": ("cassandra.cache", "SemanticCache"),
    "SemanticCacheLayer": ("cassandra.cache", "SemanticCacheLayer"),
    # Relevance scoring
    "RelevanceScore": ("cassandra.relevance", "RelevanceScore"),
    "RelevanceScorer": ("cassandra.relevance", "RelevanceScorer"),
    "BM25Scorer": ("cassandra.relevance", "BM25Scorer"),
    "EmbeddingScorer": ("cassandra.relevance", "EmbeddingScorer"),
    "HybridScorer": ("cassandra.relevance", "HybridScorer"),
    "create_scorer": ("cassandra.relevance", "create_scorer"),
    "embedding_available": ("cassandra.relevance", "embedding_available"),
    # Utilities
    "Tokenizer": ("cassandra.tokenizer", "Tokenizer"),
    "count_tokens_text": ("cassandra.tokenizer", "count_tokens_text"),
    "count_tokens_messages": ("cassandra.tokenizer", "count_tokens_messages"),
    "generate_report": ("cassandra.reporting", "generate_report"),
    # Observability
    "CassandraOtelMetrics": ("cassandra.observability", "CassandraOtelMetrics"),
    "CassandraTracer": ("cassandra.observability", "CassandraTracer"),
    "LangfuseTracingConfig": ("cassandra.observability", "LangfuseTracingConfig"),
    "OTelMetricsConfig": ("cassandra.observability", "OTelMetricsConfig"),
    "configure_otel_metrics": ("cassandra.observability", "configure_otel_metrics"),
    "configure_langfuse_tracing": ("cassandra.observability", "configure_langfuse_tracing"),
    "get_cassandra_tracer": ("cassandra.observability", "get_cassandra_tracer"),
    "get_langfuse_tracing_status": ("cassandra.observability", "get_langfuse_tracing_status"),
    "get_otel_metrics": ("cassandra.observability", "get_otel_metrics"),
    "get_otel_metrics_status": ("cassandra.observability", "get_otel_metrics_status"),
    "reset_cassandra_tracing": ("cassandra.observability", "reset_cassandra_tracing"),
    "reset_otel_metrics": ("cassandra.observability", "reset_otel_metrics"),
    # One-function API
    "compress": ("cassandra.compress", "compress"),
    "compress_spreadsheet": ("cassandra.compress", "compress_spreadsheet"),
    # Hooks
    "CompressionHooks": ("cassandra.hooks", "CompressionHooks"),
    "CompressContext": ("cassandra.hooks", "CompressContext"),
    "CompressEvent": ("cassandra.hooks", "CompressEvent"),
    # Canonical pipeline
    "PipelineStage": ("cassandra.pipeline", "PipelineStage"),
    "PipelineEvent": ("cassandra.pipeline", "PipelineEvent"),
    "PipelineExtensionManager": ("cassandra.pipeline", "PipelineExtensionManager"),
    "CANONICAL_PIPELINE_STAGES": ("cassandra.pipeline", "CANONICAL_PIPELINE_STAGES"),
    # Shared context
    "SharedContext": ("cassandra.shared_context", "SharedContext"),
}

# Memory remains optional and preserves the long-standing behavior of exposing
# `None` when the extra dependencies are not installed.
_OPTIONAL_EXPORTS = {
    "with_memory": ("cassandra.memory", "with_memory"),
    "Memory": ("cassandra.memory", "Memory"),
    "ScopeLevel": ("cassandra.memory", "ScopeLevel"),
    "HierarchicalMemory": ("cassandra.memory", "HierarchicalMemory"),
    "MemoryConfig": ("cassandra.memory", "MemoryConfig"),
    "EmbedderBackend": ("cassandra.memory", "EmbedderBackend"),
}


def __getattr__(name: str) -> Any:
    """Resolve package exports lazily while preserving legacy import paths."""
    module_attr = _LAZY_EXPORTS.get(name)
    if module_attr is not None:
        module_name, attr_name = module_attr
        value = getattr(import_module(module_name), attr_name)
        globals()[name] = value
        return value

    optional_module_attr = _OPTIONAL_EXPORTS.get(name)
    if optional_module_attr is not None:
        module_name, attr_name = optional_module_attr
        try:
            value = getattr(import_module(module_name), attr_name)
        except ImportError:
            value = None
        globals()[name] = value
        return value

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(set(globals()) | set(__all__))

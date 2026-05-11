"""Cassandra integrations with popular LLM frameworks.

Available integrations:

LangChain (pip install cassandra[langchain]):
    - CassandraChatModel: Drop-in wrapper for any LangChain chat model
    - CassandraChatMessageHistory: Automatic conversation compression
    - CassandraDocumentCompressor: Relevance-based document filtering
    - CassandraToolWrapper: Tool output compression for agents
    - StreamingMetricsTracker: Token counting during streaming
    - CassandraLangSmithCallbackHandler: LangSmith trace enrichment

Agno (pip install agno):
    - CassandraAgnoModel: Drop-in wrapper for any Agno model
    - CassandraPreHook/CassandraPostHook: Agent-level hooks for tracking
    - create_cassandra_hooks: Convenience function to create hook pairs

MCP (Model Context Protocol):
    - CassandraMCPCompressor: Compress MCP tool results
    - compress_tool_result: Simple function for tool compression

Example:
    # LangChain integration
    from cassandra.integrations import CassandraChatModel
    # or explicitly:
    from cassandra.integrations.langchain import CassandraChatModel

    # Agno integration
    from cassandra.integrations.agno import CassandraAgnoModel
    # or explicitly:
    from cassandra.integrations.agno import CassandraAgnoModel

    # MCP integration
    from cassandra.integrations import compress_tool_result
    # or explicitly:
    from cassandra.integrations.mcp import compress_tool_result
"""

# Re-export from langchain subpackage for backwards compatibility
from .langchain import (
    # Retrievers
    CompressionMetrics,
    # Core
    CassandraCallbackHandler,
    # Memory
    CassandraChatMessageHistory,
    CassandraChatModel,
    CassandraDocumentCompressor,
    # LangSmith
    CassandraLangSmithCallbackHandler,
    CassandraRunnable,
    # Agents
    CassandraToolWrapper,
    OptimizationMetrics,
    # Streaming
    StreamingMetrics,
    StreamingMetricsCallback,
    StreamingMetricsTracker,
    ToolCompressionMetrics,
    ToolMetricsCollector,
    # Provider Detection
    detect_provider,
    get_cassandra_provider,
    get_model_name_from_langchain,
    get_tool_metrics,
    is_langsmith_available,
    is_langsmith_tracing_enabled,
    langchain_available,
    optimize_messages,
    reset_tool_metrics,
    track_async_streaming_response,
    track_streaming_response,
    wrap_tools_with_cassandra,
)

# Re-export from mcp subpackage for backwards compatibility
from .mcp import (
    DEFAULT_MCP_PROFILES,
    CassandraMCPClientWrapper,
    CassandraMCPCompressor,
    MCPCompressionResult,
    MCPToolProfile,
    compress_tool_result,
    compress_tool_result_with_metrics,
    create_cassandra_mcp_proxy,
)

# Re-export from agno subpackage (optional dependency)
try:
    from .agno import (
        CassandraAgnoModel,
        CassandraPostHook,
        CassandraPreHook,
        agno_available,
        create_cassandra_hooks,
        get_model_name_from_agno,
    )
    from .agno import OptimizationMetrics as AgnoOptimizationMetrics
    from .agno import get_cassandra_provider as get_agno_provider
    from .agno import optimize_messages as optimize_agno_messages

    _AGNO_AVAILABLE = True
except ImportError:
    _AGNO_AVAILABLE = False

__all__ = [
    # LangChain Core
    "CassandraChatModel",
    "CassandraCallbackHandler",
    "CassandraRunnable",
    "OptimizationMetrics",
    "optimize_messages",
    "langchain_available",
    # Provider Detection
    "detect_provider",
    "get_cassandra_provider",
    "get_model_name_from_langchain",
    # Memory
    "CassandraChatMessageHistory",
    # Retrievers
    "CassandraDocumentCompressor",
    "CompressionMetrics",
    # Agents
    "CassandraToolWrapper",
    "ToolCompressionMetrics",
    "ToolMetricsCollector",
    "wrap_tools_with_cassandra",
    "get_tool_metrics",
    "reset_tool_metrics",
    # LangSmith
    "CassandraLangSmithCallbackHandler",
    "is_langsmith_available",
    "is_langsmith_tracing_enabled",
    # Streaming
    "StreamingMetricsTracker",
    "StreamingMetricsCallback",
    "StreamingMetrics",
    "track_streaming_response",
    "track_async_streaming_response",
    # MCP
    "CassandraMCPCompressor",
    "CassandraMCPClientWrapper",
    "MCPCompressionResult",
    "MCPToolProfile",
    "compress_tool_result",
    "compress_tool_result_with_metrics",
    "create_cassandra_mcp_proxy",
    "DEFAULT_MCP_PROFILES",
    # Agno
    "CassandraAgnoModel",
    "CassandraPreHook",
    "CassandraPostHook",
    "agno_available",
    "create_cassandra_hooks",
    "get_agno_provider",
    "get_model_name_from_agno",
    "AgnoOptimizationMetrics",
    "optimize_agno_messages",
]

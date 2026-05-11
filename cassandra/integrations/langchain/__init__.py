"""LangChain integration for Cassandra.

This package provides seamless integration with LangChain, including:
- CassandraChatModel: Drop-in wrapper for any LangChain chat model
- CassandraChatMessageHistory: Automatic conversation compression
- CassandraDocumentCompressor: Relevance-based document filtering
- CassandraToolWrapper: Tool output compression for agents
- StreamingMetricsTracker: Token counting during streaming
- CassandraLangSmithCallbackHandler: LangSmith trace enrichment
- compress_tool_messages: LangGraph pre-model hook for ToolMessage compression
- create_compress_tool_messages_node: LangGraph node factory

Example:
    from langchain_openai import ChatOpenAI
    from cassandra.integrations.langchain import CassandraChatModel

    # Wrap any LangChain model
    llm = CassandraChatModel(ChatOpenAI(model="gpt-4o"))

    # Use like normal - optimization happens automatically
    response = llm.invoke("Hello!")

Install: pip install cassandra[langchain]
"""

# Agent tool wrapping
from .agents import (
    CassandraToolWrapper,
    ToolCompressionMetrics,
    ToolMetricsCollector,
    get_tool_metrics,
    reset_tool_metrics,
    wrap_tools_with_cassandra,
)

# Core chat model wrapper
from .chat_model import (
    CassandraCallbackHandler,
    CassandraChatModel,
    CassandraRunnable,
    OptimizationMetrics,
    langchain_available,
    optimize_messages,
)

# LangGraph integration
from .langgraph import (
    CompressToolMessagesConfig,
    CompressToolMessagesResult,
    ToolMessageCompressionMetrics,
    compress_tool_messages,
    create_compress_tool_messages_node,
)

# LangSmith integration
from .langsmith import (
    CassandraLangSmithCallbackHandler,
    is_langsmith_available,
    is_langsmith_tracing_enabled,
)

# Memory integration
from .memory import CassandraChatMessageHistory

# Provider auto-detection
from .providers import (
    detect_provider,
    get_cassandra_provider,
    get_model_name_from_langchain,
)

# Retriever integration
from .retriever import CompressionMetrics, CassandraDocumentCompressor

# Streaming metrics
from .streaming import (
    StreamingMetrics,
    StreamingMetricsCallback,
    StreamingMetricsTracker,
    track_async_streaming_response,
    track_streaming_response,
)

__all__ = [
    # Core
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
    # LangGraph
    "compress_tool_messages",
    "create_compress_tool_messages_node",
    "CompressToolMessagesConfig",
    "CompressToolMessagesResult",
    "ToolMessageCompressionMetrics",
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
]

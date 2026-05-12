"""Strands Agents integration for Cassandra SDK.

This module provides seamless integration with Strands Agents,
enabling automatic context optimization for Strands agents.

Components:
1. CassandraStrandsModel - Wraps any Strands model to apply Cassandra transforms
2. CassandraHookProvider - Hook provider for Strands agents
3. get_cassandra_provider - Detects appropriate provider for a Strands model
4. get_model_name_from_strands - Extracts model name from a Strands model

Example:
    from strands import Agent
    from strands.models import BedrockModel
    from cassandra.integrations.strands import CassandraStrandsModel

    # Wrap any Strands model
    model = BedrockModel(model_id="anthropic.claude-3-5-sonnet-20241022-v2:0")
    optimized_model = CassandraStrandsModel(model)

    # Use with agent
    agent = Agent(model=optimized_model)
    response = agent("Hello!")
"""

from __future__ import annotations

import importlib.util
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .bundle import CassandraBundle
    from .hooks import CassandraHookProvider
    from .model import CassandraStrandsModel, OptimizationMetrics, optimize_messages
    from .providers import get_cassandra_provider, get_model_name_from_strands


def strands_available() -> bool:
    """Check if strands-agents is installed and available.

    Returns:
        True if strands-agents package is available, False otherwise.
    """
    return importlib.util.find_spec("strands") is not None


# Lazy imports to avoid import errors when strands is not installed
def __getattr__(name: str) -> Any:
    """Lazy import of integration components."""
    if name == "CassandraHookProvider":
        from .hooks import CassandraHookProvider

        return CassandraHookProvider
    elif name == "CassandraStrandsModel":
        from .model import CassandraStrandsModel

        return CassandraStrandsModel
    elif name == "OptimizationMetrics":
        from .model import OptimizationMetrics

        return OptimizationMetrics
    elif name == "optimize_messages":
        from .model import optimize_messages

        return optimize_messages
    elif name == "get_cassandra_provider":
        from .providers import get_cassandra_provider

        return get_cassandra_provider
    elif name == "get_model_name_from_strands":
        from .providers import get_model_name_from_strands

        return get_model_name_from_strands
    elif name == "CassandraBundle":
        from .bundle import CassandraBundle

        return CassandraBundle
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


__all__ = [
    # Availability check
    "strands_available",
    # Hook provider
    "CassandraHookProvider",
    # Model wrapper
    "CassandraStrandsModel",
    "OptimizationMetrics",
    "optimize_messages",
    # Provider detection
    "get_cassandra_provider",
    "get_model_name_from_strands",
    # One-helper MCP + hook wiring (Cassandra + tokensave/Serena + RTK-equivalent)
    "CassandraBundle",
]

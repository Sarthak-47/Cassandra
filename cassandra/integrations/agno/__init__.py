"""Agno integration for Cassandra SDK.

This module provides seamless integration with Agno (formerly Phidata),
enabling automatic context optimization for Agno agents.

Components:
1. CassandraAgnoModel - Wraps any Agno model to apply Cassandra transforms
2. create_cassandra_hooks - Creates pre/post hooks for Agno agents
3. optimize_messages - Standalone function for manual optimization

Example:
    from agno.agent import Agent
    from agno.models.openai import OpenAIChat
    from cassandra.integrations.agno import CassandraAgnoModel

    # Wrap any Agno model
    model = OpenAIChat(id="gpt-4o")
    optimized_model = CassandraAgnoModel(model)

    # Use with agent
    agent = Agent(model=optimized_model)
    response = agent.run("Hello!")
"""

from .hooks import (
    CassandraPostHook,
    CassandraPreHook,
    HookMetrics,
    create_cassandra_hooks,
)
from .model import (
    CassandraAgnoModel,
    OptimizationMetrics,
    agno_available,
    optimize_messages,
)
from .providers import get_cassandra_provider, get_model_name_from_agno

__all__ = [
    # Model wrapper
    "CassandraAgnoModel",
    "OptimizationMetrics",
    "agno_available",
    "optimize_messages",
    # Hooks
    "create_cassandra_hooks",
    "CassandraPreHook",
    "CassandraPostHook",
    "HookMetrics",
    # Provider detection
    "get_cassandra_provider",
    "get_model_name_from_agno",
]

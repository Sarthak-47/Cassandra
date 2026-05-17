"""Handler mixins for CassandraProxy.

Each mixin class contains methods extracted from CassandraProxy that handle
requests for a specific provider or concern. The mixins rely on CassandraProxy's
__init__ for all self.* attributes (duck typing).
"""

from cassandra.proxy.handlers.anthropic import AnthropicHandlerMixin
from cassandra.proxy.handlers.batch import BatchHandlerMixin
from cassandra.proxy.handlers.bedrock import BedrockHandlerMixin
from cassandra.proxy.handlers.gemini import GeminiHandlerMixin
from cassandra.proxy.handlers.openai import OpenAIHandlerMixin
from cassandra.proxy.handlers.streaming import StreamingMixin

__all__ = [
    "AnthropicHandlerMixin",
    "BatchHandlerMixin",
    "BedrockHandlerMixin",
    "GeminiHandlerMixin",
    "OpenAIHandlerMixin",
    "StreamingMixin",
]

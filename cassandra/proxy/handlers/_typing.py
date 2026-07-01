"""Type-only structural host for the ``*HandlerMixin`` classes.

``CassandraProxy`` (``cassandra/proxy/server.py``) composes ``StreamingMixin``,
``AnthropicHandlerMixin``, ``OpenAIHandlerMixin``, ``GeminiHandlerMixin``,
``BatchHandlerMixin``, and ``BedrockHandlerMixin`` via multiple inheritance.
Each mixin only implements a slice of the proxy's behavior and calls back
into attributes/methods that ``CassandraProxy.__init__`` or a *sibling*
mixin provides on the same ``self``. mypy checks each mixin file in
isolation, so it can't see those sibling members without help.

``ProxyHandlerHost`` declares that structural surface. Mixins add it as a
``TYPE_CHECKING``-only base (see the ``if TYPE_CHECKING: ... else: ... =
object`` pattern each mixin file uses) — this has zero effect at runtime,
it only gives mypy something to check attribute access against.
"""

from __future__ import annotations

from typing import Any, Protocol


class ProxyHandlerHost(Protocol):
    # Class/instance attributes set on CassandraProxy.
    ANTHROPIC_API_URL: str
    OPENAI_API_URL: str
    GEMINI_API_URL: str
    config: Any
    http_client: Any
    http_client_h1: Any
    metrics: Any
    cache: Any
    security: Any
    logger: Any
    ccr_response_handler: Any
    ccr_context_tracker: Any
    memory_handler: Any
    anthropic_provider: Any
    anthropic_pipeline: Any
    anthropic_backend: Any
    openai_provider: Any
    openai_pipeline: Any
    cost_tracker: Any
    rate_limiter: Any
    session_tracker_store: Any
    traffic_learner: Any
    usage_reporter: Any
    _active_streams: Any
    _turn_counter: int
    _background_compressor: Any
    _background_compression_min_tokens: int

    # Methods implemented by one mixin and called from a sibling mixin.
    def _get_session_key(self, *args: Any, **kwargs: Any) -> Any: ...
    def _queue_mid_turn_message(self, *args: Any, **kwargs: Any) -> Any: ...
    def _stream_response(self, *args: Any, **kwargs: Any) -> Any: ...
    def _stream_response_bedrock(self, *args: Any, **kwargs: Any) -> Any: ...
    def _stream_openai_via_backend(self, *args: Any, **kwargs: Any) -> Any: ...
    def _retry_request(self, *args: Any, **kwargs: Any) -> Any: ...
    def _record_request_outcome(self, *args: Any, **kwargs: Any) -> Any: ...
    def _next_request_id(self, *args: Any, **kwargs: Any) -> Any: ...
    def _run_compression_in_executor(self, *args: Any, **kwargs: Any) -> Any: ...
    def _get_compression_cache(self, *args: Any, **kwargs: Any) -> Any: ...
    def _assistant_message_from_response_json(self, *args: Any, **kwargs: Any) -> Any: ...
    def _messages_to_gemini_contents(self, *args: Any, **kwargs: Any) -> Any: ...
    def _gemini_contents_to_messages(self, *args: Any, **kwargs: Any) -> Any: ...
    def handle_passthrough(self, *args: Any, **kwargs: Any) -> Any: ...

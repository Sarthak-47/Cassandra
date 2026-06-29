/**
 * Tests for error hierarchy — matches Python test_exceptions.py patterns.
 */
import { describe, it, expect } from "vitest";
import {
  CassandraError,
  CassandraConnectionError,
  CassandraAuthError,
  CassandraCompressError,
  ConfigurationError,
  ProviderError,
  StorageError,
  TokenizationError,
  CacheError,
  ValidationError,
  TransformError,
  mapProxyError,
} from "../src/errors.js";

describe("CassandraError", () => {
  it("has correct name and message", () => {
    const err = new CassandraError("test error");
    expect(err.name).toBe("CassandraError");
    expect(err.message).toBe("test error");
    expect(err).toBeInstanceOf(Error);
  });

  it("supports details", () => {
    const err = new CassandraError("test", { key: "value" });
    expect(err.details).toEqual({ key: "value" });
  });

  it("details are optional", () => {
    const err = new CassandraError("test");
    expect(err.details).toBeUndefined();
  });
});

describe("CassandraConnectionError", () => {
  it("inherits from CassandraError", () => {
    const err = new CassandraConnectionError("connection failed");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("CassandraConnectionError");
  });
});

describe("CassandraAuthError", () => {
  it("inherits from CassandraError", () => {
    const err = new CassandraAuthError("unauthorized");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("CassandraAuthError");
  });
});

describe("CassandraCompressError", () => {
  it("includes statusCode and errorType", () => {
    const err = new CassandraCompressError(500, "compression_error", "failed");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("CassandraCompressError");
    expect(err.statusCode).toBe(500);
    expect(err.errorType).toBe("compression_error");
    expect(err.message).toBe("failed");
  });
});

describe("ConfigurationError", () => {
  it("inherits from CassandraError", () => {
    const err = new ConfigurationError("bad config");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("ConfigurationError");
  });
});

describe("ProviderError", () => {
  it("inherits from CassandraError", () => {
    const err = new ProviderError("provider failed");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("ProviderError");
  });
});

describe("StorageError", () => {
  it("inherits from CassandraError", () => {
    const err = new StorageError("db error");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("StorageError");
  });
});

describe("TokenizationError", () => {
  it("inherits from CassandraError", () => {
    const err = new TokenizationError("token count failed");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("TokenizationError");
  });
});

describe("CacheError", () => {
  it("inherits from CassandraError", () => {
    const err = new CacheError("cache miss");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("CacheError");
  });
});

describe("ValidationError", () => {
  it("inherits from CassandraError", () => {
    const err = new ValidationError("invalid setup");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("ValidationError");
  });
});

describe("TransformError", () => {
  it("inherits from CassandraError", () => {
    const err = new TransformError("transform failed");
    expect(err).toBeInstanceOf(CassandraError);
    expect(err.name).toBe("TransformError");
  });
});

describe("mapProxyError", () => {
  it("maps 401 to CassandraAuthError", () => {
    const err = mapProxyError(401, "auth_error", "unauthorized");
    expect(err).toBeInstanceOf(CassandraAuthError);
    expect(err.message).toBe("unauthorized");
  });

  it("maps configuration_error type", () => {
    const err = mapProxyError(400, "configuration_error", "bad config");
    expect(err).toBeInstanceOf(ConfigurationError);
  });

  it("maps provider_error type", () => {
    const err = mapProxyError(502, "provider_error", "upstream failed");
    expect(err).toBeInstanceOf(ProviderError);
  });

  it("maps storage_error type", () => {
    const err = mapProxyError(500, "storage_error", "db down");
    expect(err).toBeInstanceOf(StorageError);
  });

  it("maps tokenization_error type", () => {
    const err = mapProxyError(500, "tokenization_error", "bad tokens");
    expect(err).toBeInstanceOf(TokenizationError);
  });

  it("maps cache_error type", () => {
    const err = mapProxyError(500, "cache_error", "cache issue");
    expect(err).toBeInstanceOf(CacheError);
  });

  it("maps validation_error type", () => {
    const err = mapProxyError(422, "validation_error", "invalid");
    expect(err).toBeInstanceOf(ValidationError);
  });

  it("maps transform_error type", () => {
    const err = mapProxyError(500, "transform_error", "transform broke");
    expect(err).toBeInstanceOf(TransformError);
  });

  it("falls back to CassandraCompressError for unknown types", () => {
    const err = mapProxyError(500, "unknown_error", "something broke");
    expect(err).toBeInstanceOf(CassandraCompressError);
  });
});

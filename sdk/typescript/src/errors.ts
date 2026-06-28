/**
 * Error hierarchy matching Python cassandra.exceptions.
 */

export class CassandraError extends Error {
  details?: Record<string, any>;

  constructor(message: string, details?: Record<string, any>) {
    super(message);
    this.name = "CassandraError";
    this.details = details;
  }
}

export class CassandraConnectionError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "CassandraConnectionError";
  }
}

export class CassandraAuthError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "CassandraAuthError";
  }
}

export class CassandraCompressError extends CassandraError {
  statusCode: number;
  errorType: string;

  constructor(statusCode: number, errorType: string, message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "CassandraCompressError";
    this.statusCode = statusCode;
    this.errorType = errorType;
  }
}

export class ConfigurationError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "ConfigurationError";
  }
}

export class ProviderError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "ProviderError";
  }
}

export class StorageError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "StorageError";
  }
}

export class TokenizationError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "TokenizationError";
  }
}

export class CacheError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "CacheError";
  }
}

export class ValidationError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "ValidationError";
  }
}

export class TransformError extends CassandraError {
  constructor(message: string, details?: Record<string, any>) {
    super(message, details);
    this.name = "TransformError";
  }
}

// --- Proxy error mapping ---

const ERROR_TYPE_MAP: Record<string, new (message: string, details?: Record<string, any>) => CassandraError> = {
  configuration_error: ConfigurationError,
  provider_error: ProviderError,
  storage_error: StorageError,
  tokenization_error: TokenizationError,
  cache_error: CacheError,
  validation_error: ValidationError,
  transform_error: TransformError,
};

/**
 * Map a proxy error response to the correct CassandraError subclass.
 */
export function mapProxyError(
  status: number,
  type: string,
  message: string,
): CassandraError {
  if (status === 401) return new CassandraAuthError(message);
  const ErrorClass = ERROR_TYPE_MAP[type];
  if (ErrorClass) return new ErrorClass(message, { statusCode: status, errorType: type });
  return new CassandraCompressError(status, type, message);
}

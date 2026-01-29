# frozen_string_literal: true
# typed: strict

module Bgql
  # Error codes for SDK errors
  class ErrorCode < T::Enum
    enums do
      NetworkError = new("network_error")
      Timeout = new("timeout")
      ParseError = new("parse_error")
      ValidationError = new("validation_error")
      ExecutionError = new("execution_error")
      HttpError = new("http_error")
      NoData = new("no_data")
      Unauthorized = new("unauthorized")
      Forbidden = new("forbidden")
      NotFound = new("not_found")
      RateLimited = new("rate_limited")
      ServerError = new("server_error")
      Unknown = new("unknown")
    end

    sig { returns(T::Boolean) }
    def retryable?
      case self
      when NetworkError, Timeout, RateLimited, ServerError
        true
      else
        false
      end
    end

    sig { returns(T::Boolean) }
    def client_error?
      case self
      when ValidationError, Unauthorized, Forbidden, NotFound
        true
      else
        false
      end
    end
  end

  # SDK Error class
  class SdkError < StandardError
    extend T::Sig

    sig { returns(ErrorCode) }
    attr_reader :code

    sig { returns(T.nilable(T::Hash[Symbol, T.untyped])) }
    attr_reader :extensions

    sig { returns(T.nilable(SdkError)) }
    attr_reader :cause_error

    sig do
      params(
        code: ErrorCode,
        message: String,
        extensions: T.nilable(T::Hash[Symbol, T.untyped]),
        cause: T.nilable(SdkError)
      ).void
    end
    def initialize(code:, message:, extensions: nil, cause: nil)
      super(message)
      @code = code
      @extensions = extensions
      @cause_error = cause
    end

    sig { returns(T::Boolean) }
    def retryable?
      @code.retryable?
    end

    sig { returns(T::Boolean) }
    def client_error?
      @code.client_error?
    end
  end

  # GraphQL error from server response
  class GraphQLError
    extend T::Sig

    sig { returns(String) }
    attr_reader :message

    sig { returns(T.nilable(T::Array[T::Hash[String, T.untyped]])) }
    attr_reader :locations

    sig { returns(T.nilable(T::Array[T.any(String, Integer)])) }
    attr_reader :path

    sig { returns(T.nilable(T::Hash[String, T.untyped])) }
    attr_reader :extensions

    sig do
      params(
        message: String,
        locations: T.nilable(T::Array[T::Hash[String, T.untyped]]),
        path: T.nilable(T::Array[T.any(String, Integer)]),
        extensions: T.nilable(T::Hash[String, T.untyped])
      ).void
    end
    def initialize(message:, locations: nil, path: nil, extensions: nil)
      @message = message
      @locations = locations
      @path = path
      @extensions = extensions
    end

    sig { params(hash: T::Hash[String, T.untyped]).returns(GraphQLError) }
    def self.from_hash(hash)
      new(
        message: hash["message"] || "Unknown error",
        locations: hash["locations"],
        path: hash["path"],
        extensions: hash["extensions"]
      )
    end
  end
end

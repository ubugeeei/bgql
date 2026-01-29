# frozen_string_literal: true
# typed: strict

module Bgql
  # Type-safe context for resolvers
  #
  # @example
  #   ctx = Bgql::Context.new
  #   ctx.set(CurrentUserId, "user-123")
  #   user_id = ctx.get(CurrentUserId) # => "user-123"
  #
  class Context
    extend T::Sig

    sig { void }
    def initialize
      @data = T.let({}, T::Hash[T.class_of(ContextKey), T.untyped])
      @headers = T.let({}, T::Hash[String, String])
    end

    # Set a typed value in the context
    sig do
      type_parameters(:T)
        .params(key: T.class_of(ContextKey), value: T.type_parameter(:T))
        .returns(T.type_parameter(:T))
    end
    def set(key, value)
      @data[key] = value
      value
    end

    # Get a typed value from the context
    sig do
      type_parameters(:T)
        .params(key: T.class_of(ContextKey))
        .returns(T.nilable(T.type_parameter(:T)))
    end
    def get(key)
      @data[key]
    end

    # Get a required value from the context (raises if not present)
    sig do
      type_parameters(:T)
        .params(key: T.class_of(ContextKey))
        .returns(T.type_parameter(:T))
    end
    def require!(key)
      value = @data[key]
      raise KeyError, "Required context key not found: #{key}" if value.nil?
      value
    end

    # Check if a key exists
    sig { params(key: T.class_of(ContextKey)).returns(T::Boolean) }
    def key?(key)
      @data.key?(key)
    end

    # Set a header
    sig { params(name: String, value: String).returns(String) }
    def set_header(name, value)
      @headers[name] = value
    end

    # Get a header
    sig { params(name: String).returns(T.nilable(String)) }
    def header(name)
      @headers[name]
    end

    # Get all headers
    sig { returns(T::Hash[String, String]) }
    def headers
      @headers.dup
    end
  end

  # Base class for context keys (provides type safety via Sorbet)
  #
  # @example Define a context key
  #   class CurrentUserId < Bgql::ContextKey
  #     ValueType = type_member { { fixed: String } }
  #   end
  #
  #   class CurrentUser < Bgql::ContextKey
  #     ValueType = type_member { { fixed: User } }
  #   end
  #
  class ContextKey
    extend T::Sig
    extend T::Generic
    abstract!

    ValueType = type_member
  end

  # Built-in context keys
  class CurrentUserId < ContextKey
    ValueType = type_member { { fixed: String } }
  end

  class RequestId < ContextKey
    ValueType = type_member { { fixed: String } }
  end

  class UserRoles < ContextKey
    ValueType = type_member { { fixed: T::Array[String] } }
  end

  # Context builder for fluent API
  #
  # @example
  #   ctx = Bgql::ContextBuilder.new
  #     .with(CurrentUserId, "user-123")
  #     .with(RequestId, SecureRandom.uuid)
  #     .build
  #
  class ContextBuilder
    extend T::Sig

    sig { void }
    def initialize
      @context = T.let(Context.new, Context)
    end

    sig do
      type_parameters(:T)
        .params(key: T.class_of(ContextKey), value: T.type_parameter(:T))
        .returns(ContextBuilder)
    end
    def with(key, value)
      @context.set(key, value)
      self
    end

    sig { params(name: String, value: String).returns(ContextBuilder) }
    def with_header(name, value)
      @context.set_header(name, value)
      self
    end

    sig { returns(Context) }
    def build
      @context
    end
  end
end

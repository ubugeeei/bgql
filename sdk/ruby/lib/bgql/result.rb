# frozen_string_literal: true
# typed: strict

module Bgql
  # Result type for error handling (similar to Rust's Result)
  module Result
    extend T::Sig
    extend T::Helpers

    interface!

    sig { abstract.returns(T::Boolean) }
    def ok?; end

    sig { abstract.returns(T::Boolean) }
    def err?; end

    # Success result
    class Ok
      extend T::Sig
      extend T::Generic
      include Result

      Value = type_member

      sig { returns(Value) }
      attr_reader :value

      sig { params(value: Value).void }
      def initialize(value)
        @value = value
      end

      sig { override.returns(T::Boolean) }
      def ok?
        true
      end

      sig { override.returns(T::Boolean) }
      def err?
        false
      end

      sig do
        type_parameters(:U)
          .params(block: T.proc.params(value: Value).returns(T.type_parameter(:U)))
          .returns(Ok[T.type_parameter(:U)])
      end
      def map(&block)
        Ok.new(block.call(@value))
      end

      sig do
        type_parameters(:U)
          .params(_block: T.proc.params(error: SdkError).returns(T.type_parameter(:U)))
          .returns(Ok[Value])
      end
      def map_err(&_block)
        self
      end

      sig { params(default: Value).returns(Value) }
      def unwrap_or(default)
        @value
      end

      sig { returns(Value) }
      def unwrap!
        @value
      end
    end

    # Error result
    class Err
      extend T::Sig
      extend T::Generic
      include Result

      Value = type_member

      sig { returns(SdkError) }
      attr_reader :error

      sig { params(error: SdkError).void }
      def initialize(error)
        @error = error
      end

      sig { override.returns(T::Boolean) }
      def ok?
        false
      end

      sig { override.returns(T::Boolean) }
      def err?
        true
      end

      sig do
        type_parameters(:U)
          .params(_block: T.proc.params(value: Value).returns(T.type_parameter(:U)))
          .returns(Err[T.type_parameter(:U)])
      end
      def map(&_block)
        Err.new(@error)
      end

      sig do
        type_parameters(:U)
          .params(block: T.proc.params(error: SdkError).returns(T.type_parameter(:U)))
          .returns(T.type_parameter(:U))
      end
      def map_err(&block)
        block.call(@error)
      end

      sig { params(default: T.untyped).returns(T.untyped) }
      def unwrap_or(default)
        default
      end

      sig { returns(T.noreturn) }
      def unwrap!
        raise @error
      end
    end
  end

  # Helper methods for creating results
  module ResultHelpers
    extend T::Sig

    sig { type_parameters(:T).params(value: T.type_parameter(:T)).returns(Result::Ok[T.type_parameter(:T)]) }
    def ok(value)
      Result::Ok.new(value)
    end

    sig { type_parameters(:T).params(error: SdkError).returns(Result::Err[T.type_parameter(:T)]) }
    def err(error)
      Result::Err.new(error)
    end
  end
end

# frozen_string_literal: true
# typed: strict

module Bgql
  # Typed operation support for schema-first workflow
  #
  # @example Define a typed operation
  #   class GetUser
  #     extend T::Sig
  #     extend Bgql::TypedOperation
  #
  #     Variables = T.type_alias { { id: String } }
  #     Data = T.type_alias { { user: T.nilable(User) } }
  #
  #     OPERATION = <<~GQL
  #       query GetUser($id: ID!) {
  #         user(id: $id) {
  #           id
  #           name
  #           email
  #         }
  #       }
  #     GQL
  #   end
  #
  #   # Execute with full type safety
  #   result = client.execute_typed(GetUser, { id: "1" })
  #
  module TypedOperation
    extend T::Sig
    extend T::Helpers

    interface!

    sig { abstract.returns(String) }
    def operation; end

    sig { abstract.returns(String) }
    def operation_name; end

    sig { abstract.returns(Symbol) }
    def kind; end
  end

  # Operation kind
  module OperationKind
    QUERY = :query
    MUTATION = :mutation
    SUBSCRIPTION = :subscription
  end

  # Helper to define typed operations
  #
  # @example
  #   GetUser = Bgql.define_query(
  #     name: "GetUser",
  #     query: "query GetUser($id: ID!) { user(id: $id) { id name } }"
  #   )
  #
  module OperationBuilder
    extend T::Sig

    sig do
      params(
        name: String,
        query: String
      ).returns(T.class_of(TypedOperation))
    end
    def self.query(name:, query:)
      build_operation(name: name, query: query, kind: OperationKind::QUERY)
    end

    sig do
      params(
        name: String,
        query: String
      ).returns(T.class_of(TypedOperation))
    end
    def self.mutation(name:, query:)
      build_operation(name: name, query: query, kind: OperationKind::MUTATION)
    end

    sig do
      params(
        name: String,
        query: String,
        kind: Symbol
      ).returns(T.class_of(TypedOperation))
    end
    private_class_method def self.build_operation(name:, query:, kind:)
      Class.new do
        extend T::Sig
        include TypedOperation

        define_singleton_method(:operation) { query }
        define_singleton_method(:operation_name) { name }
        define_singleton_method(:kind) { kind }

        sig { override.returns(String) }
        def operation
          self.class.operation
        end

        sig { override.returns(String) }
        def operation_name
          self.class.operation_name
        end

        sig { override.returns(Symbol) }
        def kind
          self.class.kind
        end
      end
    end
  end

  # Extend Client with typed execution
  class Client
    sig do
      type_parameters(:V, :D)
        .params(
          operation: T.class_of(TypedOperation),
          variables: T.type_parameter(:V)
        )
        .returns(T.any(Result::Ok[T.type_parameter(:D)], Result::Err[T.type_parameter(:D)]))
    end
    def execute_typed(operation, variables)
      execute(
        query: operation.operation,
        variables: variables,
        operation_name: operation.operation_name
      )
    end
  end
end

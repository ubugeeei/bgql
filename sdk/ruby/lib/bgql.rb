# frozen_string_literal: true
# typed: strict

require "sorbet-runtime"
require "json"
require "net/http"
require "uri"

require_relative "bgql/version"
require_relative "bgql/error"
require_relative "bgql/result"
require_relative "bgql/client"
require_relative "bgql/context"
require_relative "bgql/typed"

# Better GraphQL Ruby SDK
#
# Provides strongly typed GraphQL client with Sorbet support.
#
# @example Client usage
#   client = Bgql::Client.new(url: "http://localhost:4000/bgql")
#
#   result = client.execute(
#     query: "query GetUser($id: ID!) { user(id: $id) { id name } }",
#     variables: { id: "1" }
#   )
#
#   case result
#   when Bgql::Result::Ok
#     puts result.value["user"]["name"]
#   when Bgql::Result::Err
#     puts "Error: #{result.error.message}"
#   end
#
module Bgql
  extend T::Sig

  class << self
    extend T::Sig

    # Create a new client
    sig { params(url: String, options: T::Hash[Symbol, T.untyped]).returns(Client) }
    def client(url, **options)
      Client.new(url: url, **options)
    end
  end
end

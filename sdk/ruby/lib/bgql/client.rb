# frozen_string_literal: true
# typed: strict

module Bgql
  # GraphQL Client
  #
  # @example
  #   client = Bgql::Client.new(url: "http://localhost:4000/bgql")
  #   result = client.execute(query: "{ users { id name } }")
  #
  class Client
    extend T::Sig
    include ResultHelpers

    sig { returns(String) }
    attr_reader :url

    sig { returns(Integer) }
    attr_reader :timeout

    sig { returns(Integer) }
    attr_reader :max_retries

    sig { returns(T::Hash[String, String]) }
    attr_reader :headers

    sig do
      params(
        url: String,
        timeout: Integer,
        max_retries: Integer,
        headers: T::Hash[String, String]
      ).void
    end
    def initialize(url:, timeout: 30, max_retries: 3, headers: {})
      @url = url
      @timeout = timeout
      @max_retries = max_retries
      @headers = T.let(
        { "Content-Type" => "application/json" }.merge(headers),
        T::Hash[String, String]
      )
    end

    # Execute a GraphQL operation
    sig do
      params(
        query: String,
        variables: T.nilable(T::Hash[String, T.untyped]),
        operation_name: T.nilable(String)
      ).returns(T.any(Result::Ok[T::Hash[String, T.untyped]], Result::Err[T::Hash[String, T.untyped]]))
    end
    def execute(query:, variables: nil, operation_name: nil)
      body = {
        query: query,
        variables: variables,
        operationName: operation_name
      }.compact

      response = execute_with_retry(body)

      case response
      when Result::Ok
        data = response.value
        if data["errors"] && !data["errors"].empty?
          first_error = data["errors"].first
          return err(SdkError.new(
            code: ErrorCode::ExecutionError,
            message: first_error["message"] || "Unknown error",
            extensions: { graphql_errors: data["errors"] }
          ))
        end

        if data["data"].nil?
          return err(SdkError.new(
            code: ErrorCode::NoData,
            message: "No data in response"
          ))
        end

        ok(data["data"])
      when Result::Err
        response
      end
    end

    private

    sig do
      params(body: T::Hash[Symbol, T.untyped])
        .returns(T.any(Result::Ok[T::Hash[String, T.untyped]], Result::Err[T::Hash[String, T.untyped]]))
    end
    def execute_with_retry(body)
      last_error = T.let(nil, T.nilable(SdkError))

      (@max_retries + 1).times do |attempt|
        sleep(0.1 * (2 ** attempt)) if attempt > 0

        result = do_request(body)

        case result
        when Result::Ok
          return result
        when Result::Err
          last_error = result.error
          return result unless result.error.retryable?
        end
      end

      err(last_error || SdkError.new(
        code: ErrorCode::Unknown,
        message: "Request failed after retries"
      ))
    end

    sig do
      params(body: T::Hash[Symbol, T.untyped])
        .returns(T.any(Result::Ok[T::Hash[String, T.untyped]], Result::Err[T::Hash[String, T.untyped]]))
    end
    def do_request(body)
      uri = URI.parse(@url)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == "https"
      http.open_timeout = @timeout
      http.read_timeout = @timeout

      request = Net::HTTP::Post.new(uri.request_uri)
      @headers.each { |key, value| request[key] = value }
      request.body = JSON.generate(body)

      response = http.request(request)

      unless response.is_a?(Net::HTTPSuccess)
        return err(SdkError.new(
          code: ErrorCode::HttpError,
          message: "HTTP #{response.code}",
          extensions: { status: response.code.to_i }
        ))
      end

      data = JSON.parse(response.body)
      ok(data)
    rescue Net::OpenTimeout, Net::ReadTimeout
      err(SdkError.new(
        code: ErrorCode::Timeout,
        message: "Request timed out"
      ))
    rescue SocketError, Errno::ECONNREFUSED => e
      err(SdkError.new(
        code: ErrorCode::NetworkError,
        message: e.message
      ))
    rescue JSON::ParserError => e
      err(SdkError.new(
        code: ErrorCode::ParseError,
        message: "Failed to parse response: #{e.message}"
      ))
    end
  end
end

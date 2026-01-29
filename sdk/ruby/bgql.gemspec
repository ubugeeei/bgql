# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "bgql"
  spec.version = "0.1.0"
  spec.authors = ["ubugeeei"]
  spec.email = ["ubugeeei@example.com"]

  spec.summary = "Strongly typed GraphQL SDK for Better GraphQL"
  spec.description = "Type-safe GraphQL client and server SDK with strong typing support"
  spec.homepage = "https://github.com/ubugeeei/bgql"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/blob/main/CHANGELOG.md"

  spec.files = Dir.glob("lib/**/*") + ["README.md", "LICENSE"]
  spec.require_paths = ["lib"]

  spec.add_dependency "json", "~> 2.0"
  spec.add_dependency "net-http", "~> 0.4"
  spec.add_dependency "sorbet-runtime", "~> 0.5"

  spec.add_development_dependency "rspec", "~> 3.0"
  spec.add_development_dependency "sorbet", "~> 0.5"
  spec.add_development_dependency "tapioca", "~> 0.11"
end

//! Command-line interface for Better GraphQL.
//!
//! # Usage
//!
//! ```bash
//! # Initialize a new project
//! bgql init my-api
//!
//! # Validate a schema
//! bgql check schema.bgql
//!
//! # Format files
//! bgql fmt schema.bgql
//!
//! # Generate TypeScript types
//! bgql codegen --lang typescript schema.bgql
//!
//! # Start the development server
//! bgql dev
//!
//! # Build for production
//! bgql build
//!
//! # Watch for changes
//! bgql watch
//!
//! # Start the language server
//! bgql lsp
//! ```

use bgql_core::Interner;
use bgql_syntax::{parse, FormatOptions};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "bgql")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProjectTemplate {
    /// Minimal project with basic schema
    Minimal,
    /// Full-stack project with server and client
    Fullstack,
    /// Server-only project
    Server,
    /// API-first project with OpenAPI integration
    Api,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CodegenLanguage {
    Typescript,
    Rust,
    Go,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum IdeTarget {
    /// Zed editor
    Zed,
    /// Visual Studio Code
    Vscode,
    /// Neovim
    Neovim,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Better GraphQL project
    Init {
        /// Project name
        name: String,

        /// Project template
        #[arg(short, long, value_enum, default_value = "minimal")]
        template: ProjectTemplate,

        /// Use TypeScript (for Node.js projects)
        #[arg(long)]
        typescript: bool,

        /// Skip git initialization
        #[arg(long)]
        no_git: bool,
    },

    /// Check GraphQL files for errors
    Check {
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Enable strict mode (treat warnings as errors)
        #[arg(long)]
        strict: bool,

        /// Check query complexity
        #[arg(long)]
        complexity: bool,

        /// Maximum allowed query depth
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },

    /// Format GraphQL files
    #[command(alias = "format")]
    Fmt {
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Check if files are formatted (don't modify)
        #[arg(long)]
        check: bool,

        /// Indentation size
        #[arg(long, default_value = "2")]
        indent: usize,

        /// Use tabs instead of spaces
        #[arg(long)]
        tabs: bool,
    },

    /// Generate code from GraphQL schema
    Codegen {
        /// Schema file path
        #[arg(required = true)]
        schema: PathBuf,

        /// Output file or directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target language
        #[arg(short, long, value_enum, default_value = "typescript")]
        lang: CodegenLanguage,

        /// Watch for changes and regenerate
        #[arg(short, long)]
        watch: bool,
    },

    /// Start the development server
    Dev {
        /// Schema file path
        #[arg(default_value = "schema.bgql")]
        schema: PathBuf,

        /// Port to listen on
        #[arg(short, long, default_value = "4000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable GraphQL Playground
        #[arg(long)]
        playground: bool,

        /// Enable hot reload
        #[arg(long)]
        hot_reload: bool,
    },

    /// Build schema for production
    Build {
        /// Schema file path
        #[arg(default_value = "schema.bgql")]
        schema: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,

        /// Minify output
        #[arg(long)]
        minify: bool,

        /// Generate introspection schema
        #[arg(long)]
        introspection: bool,
    },

    /// Watch files and run commands on change
    Watch {
        /// Files or directories to watch
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Command to run on change
        #[arg(short, long, default_value = "check")]
        command: String,

        /// File extensions to watch
        #[arg(short, long, default_value = "bgql,graphql")]
        extensions: String,
    },

    /// Start the language server
    Lsp,

    /// Parse a GraphQL file and print the AST
    Parse {
        /// File to parse
        file: PathBuf,

        /// Output format (pretty, json, sexp)
        #[arg(long, default_value = "pretty")]
        format: String,
    },

    /// Print version information
    Version,

    /// IDE/Editor integration
    Ide {
        /// Target IDE
        #[arg(value_enum)]
        target: IdeTarget,

        /// Install the extension
        #[arg(long)]
        install: bool,

        /// Uninstall the extension
        #[arg(long)]
        uninstall: bool,

        /// Show extension info
        #[arg(long)]
        info: bool,
    },
}

pub fn run(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init { .. } => {
            println!("Init command not yet implemented");
            Ok(0)
        }
        Commands::Check {
            files,
            strict,
            complexity: _,
            max_depth: _,
        } => check_files(&files, strict, cli.verbose),
        Commands::Fmt {
            files,
            check,
            indent,
            tabs,
        } => format_files(&files, check, indent, tabs, cli.verbose),
        Commands::Codegen {
            schema,
            output,
            lang,
            watch: _,
        } => {
            let lang_str = match lang {
                CodegenLanguage::Typescript => "typescript",
                CodegenLanguage::Rust => "rust",
                CodegenLanguage::Go => "go",
            };
            generate_code(&schema, output.as_ref(), lang_str)
        }
        Commands::Dev { .. } => {
            println!("Development server not yet implemented");
            Ok(0)
        }
        Commands::Build { .. } => {
            println!("Build command not yet implemented");
            Ok(0)
        }
        Commands::Watch { .. } => {
            println!("Watch command not yet implemented");
            Ok(0)
        }
        Commands::Lsp => {
            // Handled in main.rs
            Ok(0)
        }
        Commands::Parse { file, format } => parse_file(&file, &format),
        Commands::Version => {
            println!("bgql {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        Commands::Ide {
            target,
            install,
            uninstall,
            info,
        } => handle_ide_command(target, install, uninstall, info, cli.verbose),
    }
}

fn check_files(
    files: &[PathBuf],
    _strict: bool,
    verbose: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    let mut has_errors = false;

    for file in files {
        if verbose {
            println!("{} {}", "Checking".blue(), file.display());
        }

        let source = std::fs::read_to_string(file)?;
        let interner = Interner::new();
        let result = parse(&source, &interner);

        if result.diagnostics.has_errors() {
            has_errors = true;
            eprintln!("{} {}", "Error".red().bold(), file.display());

            for error in result.diagnostics.errors() {
                eprintln!("  {} {}", "-->".blue(), error.title);
                if let Some(msg) = &error.message {
                    eprintln!("      {}", msg);
                }
            }
        } else if verbose {
            println!("{} {}", "OK".green(), file.display());
        }
    }

    if has_errors {
        Ok(1)
    } else {
        if !files.is_empty() {
            println!(
                "{} {} file(s) checked",
                "Success:".green().bold(),
                files.len()
            );
        }
        Ok(0)
    }
}

fn format_files(
    files: &[PathBuf],
    check_only: bool,
    indent: usize,
    use_tabs: bool,
    verbose: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    let mut needs_formatting = false;

    let options = FormatOptions {
        indent_size: indent,
        use_tabs,
        ..Default::default()
    };

    for file in files {
        let source = std::fs::read_to_string(file)?;
        let interner = Interner::new();
        let result = parse(&source, &interner);

        if result.diagnostics.has_errors() {
            eprintln!("{} {} - parse error", "Error".red().bold(), file.display());
            continue;
        }

        let formatted =
            bgql_syntax::format_with_options(&result.document, &interner, options.clone());

        if check_only {
            if source != formatted {
                needs_formatting = true;
                println!("{} {}", "Would format".yellow(), file.display());
            } else if verbose {
                println!("{} {}", "OK".green(), file.display());
            }
        } else if source != formatted {
            std::fs::write(file, &formatted)?;
            println!("{} {}", "Formatted".green(), file.display());
        } else if verbose {
            println!("{} {}", "Unchanged".dimmed(), file.display());
        }
    }

    if check_only && needs_formatting {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn generate_code(
    schema_path: &Path,
    output: Option<&PathBuf>,
    lang: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(schema_path)?;
    let interner = Interner::new();
    let result = parse(&source, &interner);

    if result.diagnostics.has_errors() {
        eprintln!("{} Parse errors in schema", "Error:".red().bold());
        for error in result.diagnostics.errors() {
            eprintln!("  {}", error.title);
        }
        return Ok(1);
    }

    // Generate code based on language
    let code = match lang.to_lowercase().as_str() {
        "typescript" | "ts" => generate_typescript(&result.document, &interner),
        "rust" | "rs" => generate_rust(&result.document, &interner),
        "go" => generate_go(&result.document, &interner),
        _ => {
            eprintln!("{} Unknown language: {}", "Error:".red().bold(), lang);
            return Ok(1);
        }
    };

    match output {
        Some(path) => {
            std::fs::write(path, &code)?;
            println!("{} {}", "Generated".green(), path.display());
        }
        None => {
            println!("{}", code);
        }
    }

    Ok(0)
}

fn generate_typescript(document: &bgql_syntax::Document<'_>, interner: &Interner) -> String {
    let mut output = String::from("// Generated by Better GraphQL\n\n");

    for def in &document.definitions {
        if let bgql_syntax::Definition::Type(type_def) = def {
            match type_def {
                bgql_syntax::TypeDefinition::Object(obj) => {
                    output.push_str(&format!(
                        "export interface {} {{\n",
                        interner.get(obj.name.value)
                    ));
                    for field in &obj.fields {
                        let ts_type = type_to_typescript(&field.ty, interner);
                        output.push_str(&format!(
                            "  {}: {};\n",
                            interner.get(field.name.value),
                            ts_type
                        ));
                    }
                    output.push_str("}\n\n");
                }
                bgql_syntax::TypeDefinition::Enum(e) => {
                    let values: Vec<_> = e
                        .values
                        .iter()
                        .map(|v| format!("\"{}\"", interner.get(v.name.value)))
                        .collect();
                    output.push_str(&format!(
                        "export type {} = {};\n\n",
                        interner.get(e.name.value),
                        values.join(" | ")
                    ));
                }
                bgql_syntax::TypeDefinition::Input(inp) => {
                    output.push_str(&format!(
                        "export interface {} {{\n",
                        interner.get(inp.name.value)
                    ));
                    for field in &inp.fields {
                        let ts_type = type_to_typescript(&field.ty, interner);
                        output.push_str(&format!(
                            "  {}: {};\n",
                            interner.get(field.name.value),
                            ts_type
                        ));
                    }
                    output.push_str("}\n\n");
                }
                _ => {}
            }
        }
    }

    output
}

fn type_to_typescript(ty: &bgql_syntax::Type<'_>, interner: &Interner) -> String {
    match ty {
        bgql_syntax::Type::Named(named) => {
            let name = interner.get(named.name);
            match name.as_str() {
                "Int" | "Float" => "number".to_string(),
                "String" | "ID" => "string".to_string(),
                "Boolean" => "boolean".to_string(),
                other => other.to_string(),
            }
        }
        bgql_syntax::Type::Option(inner, _) => {
            format!("{} | null", type_to_typescript(inner, interner))
        }
        bgql_syntax::Type::List(inner, _) => {
            format!("Array<{}>", type_to_typescript(inner, interner))
        }
        bgql_syntax::Type::Generic(gen) => {
            let name = interner.get(gen.name);
            let args: Vec<_> = gen
                .arguments
                .iter()
                .map(|a| type_to_typescript(a, interner))
                .collect();
            format!("{}<{}>", name, args.join(", "))
        }
        _ => "unknown".to_string(),
    }
}

fn generate_rust(document: &bgql_syntax::Document<'_>, interner: &Interner) -> String {
    let mut output =
        String::from("// Generated by Better GraphQL\n\nuse serde::{Deserialize, Serialize};\n\n");

    for def in &document.definitions {
        if let bgql_syntax::Definition::Type(type_def) = def {
            match type_def {
                bgql_syntax::TypeDefinition::Object(obj) => {
                    output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
                    output.push_str(&format!("pub struct {} {{\n", interner.get(obj.name.value)));
                    for field in &obj.fields {
                        let rs_type = type_to_rust(&field.ty, interner);
                        output.push_str(&format!(
                            "    pub {}: {},\n",
                            interner.get(field.name.value),
                            rs_type
                        ));
                    }
                    output.push_str("}\n\n");
                }
                bgql_syntax::TypeDefinition::Enum(e) => {
                    output.push_str(
                        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n",
                    );
                    output.push_str(&format!("pub enum {} {{\n", interner.get(e.name.value)));
                    for value in &e.values {
                        output.push_str(&format!("    {},\n", interner.get(value.name.value)));
                    }
                    output.push_str("}\n\n");
                }
                _ => {}
            }
        }
    }

    output
}

fn type_to_rust(ty: &bgql_syntax::Type<'_>, interner: &Interner) -> String {
    match ty {
        bgql_syntax::Type::Named(named) => {
            let name = interner.get(named.name);
            match name.as_str() {
                "Int" => "i32".to_string(),
                "Float" => "f64".to_string(),
                "String" | "ID" => "String".to_string(),
                "Boolean" => "bool".to_string(),
                other => other.to_string(),
            }
        }
        bgql_syntax::Type::Option(inner, _) => {
            format!("Option<{}>", type_to_rust(inner, interner))
        }
        bgql_syntax::Type::List(inner, _) => {
            format!("Vec<{}>", type_to_rust(inner, interner))
        }
        _ => "()".to_string(),
    }
}

fn generate_go(document: &bgql_syntax::Document<'_>, interner: &Interner) -> String {
    let mut output = String::from("// Generated by Better GraphQL\n\npackage bgql\n\n");

    for def in &document.definitions {
        if let bgql_syntax::Definition::Type(type_def) = def {
            match type_def {
                bgql_syntax::TypeDefinition::Object(obj) => {
                    output.push_str(&format!(
                        "type {} struct {{\n",
                        interner.get(obj.name.value)
                    ));
                    for field in &obj.fields {
                        let go_type = type_to_go(&field.ty, interner);
                        let field_name = capitalize(&interner.get(field.name.value));
                        output.push_str(&format!(
                            "\t{} {} `json:\"{}\"`\n",
                            field_name,
                            go_type,
                            interner.get(field.name.value)
                        ));
                    }
                    output.push_str("}\n\n");
                }
                bgql_syntax::TypeDefinition::Enum(e) => {
                    let name = interner.get(e.name.value);
                    output.push_str(&format!("type {} string\n\nconst (\n", name));
                    for value in &e.values {
                        let val = interner.get(value.name.value);
                        output.push_str(&format!("\t{}_{} {} = \"{}\"\n", name, val, name, val));
                    }
                    output.push_str(")\n\n");
                }
                _ => {}
            }
        }
    }

    output
}

fn type_to_go(ty: &bgql_syntax::Type<'_>, interner: &Interner) -> String {
    match ty {
        bgql_syntax::Type::Named(named) => {
            let name = interner.get(named.name);
            match name.as_str() {
                "Int" => "int".to_string(),
                "Float" => "float64".to_string(),
                "String" | "ID" => "string".to_string(),
                "Boolean" => "bool".to_string(),
                other => other.to_string(),
            }
        }
        bgql_syntax::Type::Option(inner, _) => {
            format!("*{}", type_to_go(inner, interner))
        }
        bgql_syntax::Type::List(inner, _) => {
            format!("[]{}", type_to_go(inner, interner))
        }
        _ => "interface{}".to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn handle_ide_command(
    target: IdeTarget,
    install: bool,
    uninstall: bool,
    info: bool,
    verbose: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    match target {
        IdeTarget::Zed => handle_zed_command(install, uninstall, info, verbose),
        IdeTarget::Vscode => {
            println!("{} VS Code extension not yet available", "Info:".blue());
            println!("  Install from marketplace: code --install-extension bgql.bgql");
            Ok(0)
        }
        IdeTarget::Neovim => {
            println!("{} Neovim plugin not yet available", "Info:".blue());
            println!("  Add to your config:");
            println!("    require('lspconfig').bgql.setup{{}}");
            Ok(0)
        }
    }
}

fn handle_zed_command(
    install: bool,
    uninstall: bool,
    info: bool,
    verbose: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let zed_extensions_dir = PathBuf::from(&home).join(".config/zed/extensions/installed");
    let bgql_ext_dir = zed_extensions_dir.join("bgql");

    if info || (!install && !uninstall) {
        println!("{}", "Better GraphQL - Zed Extension".green().bold());
        println!();
        println!("  Extension ID: bgql");
        println!("  Version:      {}", env!("CARGO_PKG_VERSION"));
        println!("  Features:");
        println!("    - Syntax highlighting for .bgql files");
        println!("    - Language Server Protocol support");
        println!("    - Auto-completion and diagnostics");
        println!();

        if bgql_ext_dir.exists() {
            println!("  Status: {} (at {})", "Installed".green(), bgql_ext_dir.display());
        } else {
            println!("  Status: {}", "Not installed".yellow());
            println!();
            println!("  To install: bgql ide zed --install");
        }
        return Ok(0);
    }

    if uninstall {
        if bgql_ext_dir.exists() {
            std::fs::remove_dir_all(&bgql_ext_dir)?;
            println!("{} Removed bgql extension from Zed", "Success:".green().bold());
        } else {
            println!("{} bgql extension is not installed", "Info:".blue());
        }
        return Ok(0);
    }

    if install {
        // Get the extension source directory
        let extension_src = get_extension_source_dir()?;

        if verbose {
            println!("{} Extension source: {}", "Info:".blue(), extension_src.display());
        }

        // Create extensions directory if it doesn't exist
        std::fs::create_dir_all(&zed_extensions_dir)?;

        // Copy extension files
        copy_dir_recursive(&extension_src, &bgql_ext_dir)?;

        println!("{} Installed bgql extension to Zed", "Success:".green().bold());
        println!();
        println!("  Location: {}", bgql_ext_dir.display());
        println!();
        println!("  {} Restart Zed to activate the extension", "Note:".yellow());

        return Ok(0);
    }

    Ok(0)
}

fn get_extension_source_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try to find the extension in common locations
    let candidates = [
        // Development: relative to binary
        std::env::current_exe()?
            .parent()
            .unwrap_or(Path::new("."))
            .join("../../editors/zed"),
        // Development: current directory
        PathBuf::from("editors/zed"),
        // Installed: alongside binary
        std::env::current_exe()?
            .parent()
            .unwrap_or(Path::new("."))
            .join("share/bgql/editors/zed"),
        // System install
        PathBuf::from("/usr/local/share/bgql/editors/zed"),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.join("extension.toml").exists() {
            return Ok(candidate);
        }
    }

    Err("Could not find bgql Zed extension source. Make sure you're running from the bgql repository or have bgql properly installed.".into())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn parse_file(file: &Path, fmt: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(file)?;
    let interner = Interner::new();
    let result = parse(&source, &interner);

    if result.diagnostics.has_errors() {
        eprintln!("{} Parse failed", "Error:".red().bold());
        for error in result.diagnostics.errors() {
            eprintln!("  {}", error.title);
        }
        return Ok(1);
    }

    match fmt {
        "json" => {
            println!("JSON output not yet implemented");
        }
        _ => {
            println!("{:#?}", result.document);
        }
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}

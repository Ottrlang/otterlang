use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use tracing::{debug, info};

use crate::cache::{CacheBuildOptions, CacheEntry, CacheManager, CacheMetadata, CompilationInputs};
use crate::codegen::{self, build_executable, BuildArtifact, CodegenOptLevel, CodegenOptions};
use crate::lexer::{tokenize, LexerError};
use crate::parser::{parse, ParserError};
use crate::runtime::ffi;
use crate::utils::errors::{emit_diagnostics, Diagnostic};
use crate::utils::logger;
use crate::utils::profiler::{PhaseTiming, Profiler};
use crate::version::VERSION;

#[derive(Parser, Debug)]
#[command(name = "otter", version = VERSION, about = "OtterLang compiler CLI")]
pub struct OtterCli {
    #[arg(long, global = true)]
    /// Dump the token stream before parsing.
    dump_tokens: bool,

    #[arg(long, global = true)]
    /// Dump the parsed AST before code generation.
    dump_ast: bool,

    #[arg(long, global = true)]
    /// Dump the generated LLVM IR.
    dump_ir: bool,

    #[arg(long, global = true)]
    /// Display phase timing information.
    time: bool,

    #[arg(long, global = true)]
    /// Emit profiling summary for the compilation.
    profile: bool,

    #[arg(long, global = true)]
    /// Enable release mode (O3 + LTO) when building binaries.
    release: bool,

    #[arg(long, global = true)]
    /// Enable the experimental async task runtime when executing programs.
    tasks: bool,

    #[arg(long, global = true)]
    /// Emit verbose scheduler diagnostics from the task runtime.
    tasks_debug: bool,

    #[arg(long, global = true)]
    /// Trace task lifecycle events from the runtime.
    tasks_trace: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Lexes, parses, and executes the specified source file via the cached native pipeline.
    Run { path: PathBuf },
    /// Builds a native executable from the specified source file.
    Build {
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn run() -> Result<()> {
    logger::init_logging();
    ffi::bootstrap_stdlib();
    let cli = OtterCli::parse();
    match &cli.command {
        Command::Run { path } => handle_run(&cli, path),
        Command::Build { path, output } => handle_build(&cli, path, output.clone()),
    }
}

fn handle_run(cli: &OtterCli, path: &Path) -> Result<()> {
    let settings = CompilationSettings::from_cli(cli);
    let source = read_source(path)?;
    let stage = compile_pipeline(path, &source, &settings)?;

    match &stage.result {
        CompilationResult::CacheHit(entry) => {
            println!(
                "{} {}",
                "cache".green().bold(),
                format!("hit ({} bytes)", entry.metadata.binary_size)
            );
            if settings.profile {
                print_profile(&entry.metadata);
            }
            execute_binary(&entry.binary_path, &settings)?;
        }
        CompilationResult::Compiled { artifact, metadata } => {
            println!("{} {}", "building".bold(), artifact.binary.display());
            execute_binary(&artifact.binary, &settings)?;
            if settings.dump_ir {
                if let Some(ir) = &artifact.ir {
                    println!("{}", "== LLVM IR ==".bold());
                    println!("{ir}");
                }
            }
            if settings.profile {
                print_profile(metadata);
            }
        }
    }

    if settings.time {
        print_timings(&stage);
    }

    Ok(())
}

fn handle_build(cli: &OtterCli, path: &Path, output: Option<PathBuf>) -> Result<()> {
    let settings = CompilationSettings::from_cli(cli);
    let source = read_source(path)?;
    let stage = compile_pipeline(path, &source, &settings)?;

    let output_path = resolve_output_path(path, output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    let cached_binary = match &stage.result {
        CompilationResult::CacheHit(entry) => &entry.binary_path,
        CompilationResult::Compiled { artifact, .. } => &artifact.binary,
    };

    fs::copy(cached_binary, &output_path).with_context(|| {
        format!(
            "failed to copy cached binary {} to {}",
            cached_binary.display(),
            output_path.display()
        )
    })?;

    println!("{} {}", "built".green().bold(), output_path.display());

    match &stage.result {
        CompilationResult::Compiled { artifact, metadata } => {
            if settings.dump_ir {
                if let Some(ir) = &artifact.ir {
                    println!("{}", "== LLVM IR ==".bold());
                    println!("{ir}");
                }
            }
            if settings.profile {
                print_profile(metadata);
            }
        }
        CompilationResult::CacheHit(entry) => {
            if settings.profile {
                print_profile(&entry.metadata);
            }
        }
    }

    if settings.time {
        print_timings(&stage);
    }

    Ok(())
}

fn compile_pipeline(
    path: &Path,
    source: &str,
    settings: &CompilationSettings,
) -> Result<CompilationStage> {
    let cache_manager = CacheManager::new()?;
    let inputs = CompilationInputs::new(path.to_path_buf(), Vec::new());
    let cache_options = settings.cache_build_options();
    let mut profiler = Profiler::new();
    let source_id = path.display().to_string();

    let cache_key = profiler.record_phase("Fingerprint", || {
        cache_manager.fingerprint(&inputs, &cache_options, VERSION)
    })?;

    if settings.allow_cache() {
        if let Some(entry) =
            profiler.record_phase("Cache lookup", || cache_manager.lookup(&cache_key))?
        {
            debug!(cache_hit = %entry.binary_path.display());
            profiler.push_phase("Compile skipped", Duration::from_millis(0));
            return Ok(CompilationStage {
                profiler,
                result: CompilationResult::CacheHit(entry),
            });
        }
    }

    let tokens = match profiler.record_phase("Lexing", || tokenize(source)) {
        Ok(tokens) => tokens,
        Err(errors) => {
            emit_lexer_errors(&source_id, source, &errors);
            bail!("lexing failed");
        }
    };

    if settings.dump_tokens {
        println!("{}", "== Tokens ==".bold());
        for token in &tokens {
            println!("{:?} @ {:?}", token.kind, token.span);
        }
    }

    let program = match profiler.record_phase("Parsing", || parse(&tokens)) {
        Ok(program) => program,
        Err(errors) => {
            emit_parser_errors(&source_id, source, &errors);
            bail!("parsing failed");
        }
    };

    if settings.dump_ast {
        println!("{}", "== AST ==".bold());
        println!("{:#?}", program);
    }

    let codegen_options = settings.codegen_options();
    let binary_path = cache_manager.binary_path(&cache_key);

    let artifact = profiler.record_phase("LLVM Codegen", || {
        build_executable(&program, &binary_path, &codegen_options)
    })?;

    let build_duration_ms = profiler
        .phases()
        .last()
        .map(|phase| phase.duration.as_millis())
        .unwrap_or_default();

    let binary_size = CacheMetadata::binary_size(&artifact.binary)?;

    let metadata = CacheMetadata::new(
        cache_key.as_str().to_string(),
        VERSION,
        codegen::current_llvm_version(),
        canonical_or(path),
        inputs.imports.clone(),
        artifact.binary.clone(),
        binary_size,
        build_duration_ms,
        cache_options.clone(),
        Vec::new(),
    );

    cache_manager.store(&metadata)?;

    info!(compiled = %artifact.binary.display(), size = binary_size);

    Ok(CompilationStage {
        profiler,
        result: CompilationResult::Compiled { artifact, metadata },
    })
}

struct CompilationStage {
    profiler: Profiler,
    result: CompilationResult,
}

enum CompilationResult {
    CacheHit(CacheEntry),
    Compiled {
        artifact: BuildArtifact,
        metadata: CacheMetadata,
    },
}

impl CompilationStage {
    fn timings(&self) -> &[PhaseTiming] {
        self.profiler.phases()
    }
}

#[derive(Clone)]
struct CompilationSettings {
    dump_tokens: bool,
    dump_ast: bool,
    dump_ir: bool,
    time: bool,
    profile: bool,
    release: bool,
    tasks: bool,
    tasks_debug: bool,
    tasks_trace: bool,
}

impl CompilationSettings {
    fn from_cli(cli: &OtterCli) -> Self {
        Self {
            dump_tokens: cli.dump_tokens,
            dump_ast: cli.dump_ast,
            dump_ir: cli.dump_ir,
            time: cli.time,
            profile: cli.profile,
            release: cli.release,
            tasks: cli.tasks,
            tasks_debug: cli.tasks_debug,
            tasks_trace: cli.tasks_trace,
        }
    }

    fn allow_cache(&self) -> bool {
        !(self.dump_tokens || self.dump_ast || self.dump_ir)
    }

    fn cache_build_options(&self) -> CacheBuildOptions {
        CacheBuildOptions {
            release: self.release,
            lto: self.release,
            emit_ir: self.dump_ir,
        }
    }

    fn codegen_options(&self) -> CodegenOptions {
        CodegenOptions {
            emit_ir: self.dump_ir,
            opt_level: if self.release {
                CodegenOptLevel::Aggressive
            } else {
                CodegenOptLevel::Default
            },
            enable_lto: self.release,
        }
    }
}

fn read_source(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

fn resolve_output_path(path: &Path, output: Option<PathBuf>) -> PathBuf {
    output.unwrap_or_else(|| {
        let mut candidate = path.with_extension("");
        if candidate.file_name().is_none() {
            candidate = PathBuf::from("otter.out");
        }

        #[cfg(target_os = "windows")]
        {
            candidate.set_extension("exe");
        }

        candidate
    })
}

fn canonical_or(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn execute_binary(path: &Path, settings: &CompilationSettings) -> Result<()> {
    let mut command = ProcessCommand::new(path);

    if settings.tasks {
        command.env("OTTER_TASKS_DIAGNOSTICS", "1");
    }
    if settings.tasks_debug {
        command.env("OTTER_TASKS_DEBUG", "1");
    }
    if settings.tasks_trace {
        command.env("OTTER_TASKS_TRACE", "1");
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute {}", path.display()))?;

    if !status.success() {
        bail!("program exited with status {status}");
    }

    Ok(())
}

fn print_timings(stage: &CompilationStage) {
    println!("{}", "[Timing]".bold());
    let mut total = Duration::ZERO;
    for PhaseTiming { name, duration } in stage.timings() {
        println!("{:>16}: {:>6.2} ms", name, duration.as_secs_f64() * 1000.0);
        total += *duration;
    }
    println!("{:>16}: {:>6.2} ms", "Total", total.as_secs_f64() * 1000.0);
}

fn print_profile(metadata: &CacheMetadata) {
    println!("{}", "[Profile]".bold());
    println!("{:>16}: {}", "Binary", metadata.binary_path.display());
    println!("{:>16}: {} bytes", "Size", metadata.binary_size);
    println!("{:>16}: {} ms", "Build", metadata.build_time_ms);
    if let Some(version) = &metadata.llvm_version {
        println!("{:>16}: {version}", "LLVM");
    }
}

fn emit_lexer_errors(source_id: &str, source: &str, errors: &[LexerError]) {
    let diagnostics: Vec<Diagnostic> = errors
        .iter()
        .map(|err| err.to_diagnostic(source_id))
        .collect();
    emit_diagnostics(&diagnostics, source);
}

fn emit_parser_errors(source_id: &str, source: &str, errors: &[ParserError]) {
    let diagnostics: Vec<Diagnostic> = errors
        .iter()
        .map(|err| err.to_diagnostic(source_id))
        .collect();
    emit_diagnostics(&diagnostics, source);
}

//! Print the AST for a given Python file.

use std::path::PathBuf;

use anyhow::Result;

use ruff_linter::source_kind::SourceKind;
use ruff_python_ast::PySourceType;
use ruff_python_parser::{parse, ParseOptions};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Python file for which to generate the AST.
    #[arg(required = true)]
    file: PathBuf,
}

pub(crate) fn main(args: &Args) -> Result<()> {
    let source_type = PySourceType::from(&args.file);
    let source_kind = SourceKind::from_path(&args.file, source_type)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not determine source kind for file: {}",
            args.file.display()
        )
    })?;
    let python_ast =
        parse(source_kind.source_code(), ParseOptions::from(source_type))?.into_syntax();
    println!("{python_ast:#?}");
    Ok(())
}

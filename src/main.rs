// Enables lints disabled (allowed) by default to (possibly) catch more code
// errors/smells https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html

#![warn(absolute_paths_not_starting_with_crate)]
#![warn(elided_lifetimes_in_paths)]
#![warn(explicit_outlives_requirements)]
#![warn(ffi_unwind_calls)]
#![feature(strict_provenance_lints)]
#![warn(fuzzy_provenance_casts)]
#![warn(lossy_provenance_casts)]
#![warn(keyword_idents)]
#![warn(macro_use_extern_crate)]
#![warn(meta_variable_misuse)]
#![warn(missing_abi)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![feature(must_not_suspend)]
#![warn(must_not_suspend)]
#![warn(non_ascii_idents)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![warn(non_exhaustive_omitted_patterns)]
#![warn(noop_method_call)]
#![warn(rust_2021_incompatible_closure_captures)]
#![warn(rust_2021_incompatible_or_patterns)]
#![warn(rust_2021_prefixes_incompatible_syntax)]
#![warn(rust_2021_prelude_collisions)]
#![warn(single_use_lifetimes)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_lifetimes)]
#![warn(unused_macro_rules)]
#![warn(unused_qualifications)]
#![warn(unused_results)]
#![warn(dead_code)]
#![warn(variant_size_differences)]
//#![feature(stmt_expr_attributes)]
//#![feature(new_range_api)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(clippy::redundant_pub_crate)]

use std::env;
use std::path::Path;
use std::process::ExitCode;

use colored::Colorize;
use mimalloc::MiMalloc;

mod minecraft_launcher_launcher;

mod utils;

#[cfg(test)]
mod tests;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[inline]
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if let Some(binary_name) = args.first() {
        if let Some(binary_file_name) = Path::new(binary_name).file_name() {
            if let Some(argument) = args.get(1) {
                if argument == "--install" {
                    return minecraft_launcher_launcher::install(
                        &binary_file_name.to_string_lossy(),
                        &args,
                    );
                }

                eprintln!("{}{argument}", "invalid argument: ".red());

                return ExitCode::FAILURE; // Exit because providing invalid
                // arguments should not fall through
            } // No arguments given, fall through

            if binary_file_name == "minecraft-launcher" {
                // Launch the launcher
                return minecraft_launcher_launcher::launch();
            } // fall through
        } else {
            eprintln!(
                "{}",
                "warning: can't get file name path of running binary".yellow()
            );
        }
    } else {
        eprintln!("{}", "warning: can't get running binary string".yellow());
        // Fall through because we don't really need the binary name
    }

    eprintln!("{}", "error: use the --install argument or run with binary name minecraft-launcher to proceed".red());
    ExitCode::FAILURE
}

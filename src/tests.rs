use std::env;
use std::path::Path;
use std::path::PathBuf;

use rev_buf_reader::RevBufReader;
use std::io::BufRead;

use std::fs::File;

use colored::Colorize;

fn get_workspace_path() -> PathBuf {
    Path::new(".idea").join("workspace.xml")
}

fn should_test_intellij_clippy_args() -> bool {
    get_workspace_path().exists() && env::var("CLIPPY_ARGS").is_ok()
}

#[test]
fn intellij_clippy_args() {
    if should_test_intellij_clippy_args() {
        if let Ok(clippy_args) = env::var("CLIPPY_ARGS") {
            test_intellij_clippy_args0(&clippy_args);
        }
    }
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn intellij_clippy_args_should_fail() {
    if should_test_intellij_clippy_args() {
        test_intellij_clippy_args0("whatever");
    } else {
        panic!("assertion `left == right` failed"); // workaround to not
        // fail the test
    }
}

fn test_intellij_clippy_args0(args: &str) {
    let workspace_file_path = &get_workspace_path();

    let workspace_file_contents =
        lines_from_file_from_end(workspace_file_path, usize::MAX, false);

    assert!(
        !workspace_file_contents.is_empty(),
        "workspace file path is empty or can't get its contents"
    );

    for line in workspace_file_contents {
        if line
            .trim()
            .starts_with(r#"<option name="externalLinterArguments" value=""#)
        {
            assert_eq!(
                line.trim().replace('\t', "").replace(" />", "/>"),
                format!(
                    r#"<option name="externalLinterArguments" value="{args}"/>"#
                )
            );
        }
    }
}

#[inline]
#[must_use]
fn lines_from_file_from_end(
    file_path: &Path,
    limit: usize,
    print_errors: bool,
) -> Vec<String> {
    match File::open(file_path) {
        Ok(file) => {
            let buf = RevBufReader::new(file);

            buf.lines()
                .take(limit)
                .map(|operation_result| match operation_result {
                    Ok(line) => line,

                    Err(e) => {
                        eprintln!(
                            "{}{}{e}",
                            "error while processing file: ".red(),
                            file_path.to_string_lossy()
                        );

                        String::new()
                    },
                })
                .collect()
        },

        Err(e) => {
            if print_errors {
                eprintln!(
                    "{}{}{e}",
                    "can't open file: ".red(),
                    file_path.to_string_lossy()
                );
            }

            vec![]
        },
    }
}

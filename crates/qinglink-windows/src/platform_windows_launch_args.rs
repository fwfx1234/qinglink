//! Windows 命令行参数拼接与引号转义。

/// 将参数列表拼接为命令行字符串，含空格的路径自动加引号
pub(super) fn build_args_string(args: &[String]) -> String {
    args.iter()
        .map(|arg| quote_windows_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote one argument using the escaping rules understood by Windows process
/// command-line parsing. In particular, backslashes immediately before a
/// quote or the closing quote must be doubled.
fn quote_windows_arg(arg: &str) -> String {
    if !arg
        .chars()
        .any(|character| character.is_whitespace() || character == '"')
    {
        return arg.to_string();
    }
    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');
    let mut slashes = 0;
    for character in arg.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
                quoted.push('"');
                slashes = 0;
            }
            other => {
                quoted.extend(std::iter::repeat_n('\\', slashes));
                quoted.push(other);
                slashes = 0;
            }
        }
    }
    quoted.extend(std::iter::repeat_n('\\', slashes * 2));
    quoted.push('"');
    quoted
}

/// 构造 `start` 命令的目标与参数，供会话外启动使用。
pub(super) fn build_start_command_line(target: &str, args: &[String]) -> String {
    let mut command = format!("\"{target}\"");
    if !args.is_empty() {
        command.push(' ');
        command.push_str(&build_args_string(args));
    }
    command
}

#[cfg(test)]
mod launch_args_tests {
    use super::{build_args_string, quote_windows_arg};

    #[test]
    fn quotes_arguments_with_spaces_and_trailing_backslashes() {
        assert_eq!(
            quote_windows_arg(r#"C:\\Program Files\\Game\\"#),
            r#""C:\\Program Files\\Game\\\\""#
        );
        assert_eq!(
            build_args_string(&["--profile".into(), "a b".into()]),
            "--profile \"a b\""
        );
    }
}

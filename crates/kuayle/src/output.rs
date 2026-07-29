// Output formatting: human / JSON with tty auto-detection.
// 输出格式化：human / JSON，带 tty 自动检测。
//
// In human mode, output is designed for terminal display.
// In JSON mode, output is machine-readable.
// Auto mode detects whether stdout is a terminal.
// human 模式下输出专为终端显示设计。
// JSON 模式下输出为机器可读。
// auto 模式检测 stdout 是否为终端。

use serde::Serialize;

/// Output format for CLI commands.
/// CLI 命令的输出格式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    /// Human-readable terminal output.
    /// 人类可读的终端输出。
    Human,
    /// Machine-readable JSON output.
    /// 机器可读的 JSON 输出。
    Json,
}

impl Format {
    /// Determine the output format from a CLI flag and tty status.
    /// 根据 CLI flag 和 tty 状态确定输出格式。
    ///
    /// `"human"` → Human, `"json"` → Json, anything else → auto-detect tty.
    /// `"human"` → Human，`"json"` → Json，其余 → 自动检测 tty。
    pub fn from_flag(flag: &str) -> Self {
        match flag {
            "human" => Format::Human,
            "json" => Format::Json,
            _ => {
                if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                    Format::Human
                } else {
                    Format::Json
                }
            }
        }
    }

    /// Whether the format is JSON.
    /// 格式是否为 JSON。
    pub fn is_json(self) -> bool {
        matches!(self, Format::Json)
    }
}

/// Print a single value in the specified format.
/// 以指定格式打印单个值。
pub fn print_one<T: Serialize>(format: Format, value: &T) {
    match format {
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
        Format::Human => {
            // For unstructured printing, just serialize as pretty JSON for now.
            // In the future, this will have resource-specific detail views.
            // 对于非结构化打印，暂时序列化为 pretty JSON。
            // 未来将支持资源特定的详情视图。
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
    }
}

/// Print a list of values as a table (human) or JSON array (json).
/// 以表格（human）或 JSON 数组（json）打印值列表。
pub fn print_table<T: Serialize>(
    format: Format,
    items: &[T],
    headers: &[&str],
    row_fn: impl Fn(&T) -> Vec<String>,
) {
    match format {
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(items).unwrap_or_default()
            );
        }
        Format::Human => {
            if items.is_empty() {
                println!("No items found.");
                println!("没有找到项目。");
                return;
            }

            // Calculate column widths.
            // 计算列宽。
            let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
            for item in items {
                let row = row_fn(item);
                for (i, cell) in row.iter().enumerate() {
                    if i < widths.len() {
                        widths[i] = widths[i].max(cell.len());
                    }
                }
            }

            // Print header.
            // 打印表头。
            let header_line: Vec<String> = headers
                .iter()
                .enumerate()
                .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
                .collect();
            println!("{}", header_line.join("  "));

            // Print separator.
            // 打印分隔线。
            let sep: Vec<String> = widths
                .iter()
                .map(|w| "-".repeat(*w))
                .collect();
            println!("{}", sep.join("  "));

            // Print rows.
            // 打印行。
            for item in items {
                let row = row_fn(item);
                let line: Vec<String> = row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| format!("{:<width$}", cell, width = widths[i]))
                    .collect();
                println!("{}", line.join("  "));
            }

            println!("\n{} item(s)", items.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_flag_human() {
        assert_eq!(Format::from_flag("human"), Format::Human);
    }

    #[test]
    fn format_from_flag_json() {
        assert_eq!(Format::from_flag("json"), Format::Json);
    }

    #[test]
    fn format_is_json() {
        assert!(Format::Json.is_json());
        assert!(!Format::Human.is_json());
    }
}

/// Sanitizes a Windows path by removing extra backslashes and normalizing separators
pub fn sanitize_windows_path(path: &str) -> String {
    // First, clean up any forward slashes and multiple backslashes
    let cleaned = path
        .replace('/', "\\")
        .replace(r"\\", r"\")
        .trim_end_matches('\\')
        .to_string();

    // If the path is already quoted, return as is
    if cleaned.starts_with('"') && cleaned.ends_with('"') {
        return cleaned;
    }

    // Quote the path if it contains spaces
    if cleaned.contains(' ') {
        format!("\"{}\"", cleaned)
    } else {
        cleaned
    }
}

/// Converts mkdir -p command to Windows equivalent
pub fn convert_mkdir_command(command: &str) -> String {
    if !cfg!(windows) {
        return command.to_string();
    }

    if command.starts_with("mkdir -p ") {
        // Extract the path part
        let path = command.trim_start_matches("mkdir -p ").trim();
        // Remove quotes if present
        let path = path.trim_matches('"');

        // Split path into components and create each directory level
        let components: Vec<&str> = path.split('\\').collect();
        let mut commands = Vec::new();

        // Build up the path one component at a time
        let mut current_path = String::new();
        for component in components {
            if component.is_empty() {
                continue;
            }
            if current_path.is_empty() {
                current_path = component.to_string();
            } else {
                current_path = format!("{}\\{}", current_path, component);
            }
            commands.push(format!(
                "if not exist \"{}\" md \"{}\"",
                current_path, current_path
            ));
        }

        commands.join(" && ")
    } else {
        command.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_windows_path() {
        assert_eq!(
            sanitize_windows_path(r"C:\\Users\\name\\path"),
            r"C:\Users\name\path"
        );
        assert_eq!(
            sanitize_windows_path("C:/Users/name/path"),
            r"C:\Users\name\path"
        );
        assert_eq!(
            sanitize_windows_path(r"C:\Users\Program Files\path"),
            r#""C:\Users\Program Files\path""#
        );
    }

    #[test]
    fn test_convert_mkdir_command() {
        if cfg!(windows) {
            assert_eq!(
                convert_mkdir_command(r#"mkdir -p "C:\Users\test\path""#),
                r#"if not exist "C:\Users" md "C:\Users" && if not exist "C:\Users\test" md "C:\Users\test" && if not exist "C:\Users\test\path" md "C:\Users\test\path""#
            );
        }
    }
}

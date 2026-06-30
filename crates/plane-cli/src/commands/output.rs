#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn ok(stdout: String) -> Self {
        Self {
            status: 0,
            stdout,
            stderr: String::new(),
        }
    }

    pub fn err(status: i32, stderr: String) -> Self {
        Self {
            status,
            stdout: String::new(),
            stderr,
        }
    }

    pub fn emit(&self) {
        if !self.stdout.is_empty() {
            print!("{}", self.stdout);
        }
        if !self.stderr.is_empty() {
            eprint!("{}", self.stderr);
        }
    }
}

use anyhow::Result;
use std::io::{stdout, Stdout, Write};
use aws_smithy_types::Document;
use crossterm::ExecutableCommand;
use crossterm::terminal::{self, Clear};
use crossterm::style::{Color, SetForegroundColor};

#[derive(Debug)]
pub struct TerminalService {
    stdout: Stdout
}

impl TerminalService {

    pub fn new() -> Self {
        return Self {
            stdout: stdout()
        }
    }

    pub fn delete_char(&mut self) -> Result<()> {
        write!(self.stdout, "\x08 \x08")?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> Result<()> {
        self.stdout.execute(Clear(terminal::ClearType::CurrentLine))?;
        Ok(())
    }

    pub fn log_ai(&mut self, text: &str) -> Result<()>{
        self.log_info("AI:\r")?;
        self.stdout.execute(SetForegroundColor(Color::Blue))?;
        writeln!(self.stdout, "{}", text)?;
        Ok(())
    }

    pub fn log_ai_inline(&mut self, text: &str) -> Result<()>{
        self.stdout.execute(SetForegroundColor(Color::Blue))?;
        write!(self.stdout, "{}", text)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn log_user(&mut self, text: &str) -> Result<()>{
        self.stdout.execute(SetForegroundColor(Color::Green))?;
        writeln!(self.stdout, "{}", text)?;
        Ok(())
    }

    pub fn log_user_inline(&mut self, c: &char) -> Result<()>{
        self.stdout.execute(SetForegroundColor(Color::Green))?;
        write!(self.stdout, "{}", &c)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn log_error(&mut self, text: &str) -> Result<()>{
        self.stdout.execute(SetForegroundColor(Color::Red))?;
        writeln!(self.stdout, "{}", text)?;
        Ok(())
    }

    pub fn log_info(&mut self, text: &str) -> Result<()>{
        writeln!(self.stdout, "\x1b[0;90m{}", text)?;
        Ok(())
    }

    pub fn log_info_inline(&mut self, text: &str) -> Result<()>{
        write!(self.stdout, "\x1b[0;90m{}", text)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn log_tool(&mut self, tool_name: &str, tool_input: &Document) -> Result<()>{
        writeln!(self.stdout, "\x1b[0;90mTool used: {}", tool_name)?;
        writeln!(self.stdout, "\x1b[0;90mTool Input: {:?}", tool_input)?;
        Ok(())
    }

}
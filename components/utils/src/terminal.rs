use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};

use crate::Result;

#[inline]
pub fn delimiter() -> String {
    use terminal_size::{terminal_size, Width};
    if let Some((Width(w), _)) = terminal_size() {
        "─".repeat(w as usize)
    } else {
        String::from("---")
    }
}

#[inline]
pub fn clear_current_line() -> Result<()> {
    execute!(
        std::io::stdout(),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    Ok(())
}

#[inline]
pub fn clear_previous_line() -> Result<()> {
    execute!(
        std::io::stdout(),
        MoveUp(1),
        MoveToColumn(0),
        Clear(ClearType::UntilNewLine)
    )?;
    Ok(())
}

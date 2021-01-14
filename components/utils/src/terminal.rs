use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};

use crate::Result;

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

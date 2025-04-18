#[macro_export]
macro_rules! print_row {
    ($row:expr, $($arg:tt)*) => {
        execute!(
            stdout(),
            cursor::MoveTo(0, $row as u16),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            Print(format!($($arg)*)),
            cursor::MoveTo(0, $row as u16)
        )?;
    };
}
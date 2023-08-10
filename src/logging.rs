use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::LevelFilter;
use std::str::FromStr;

/// Configures fern logging library.
pub fn init(log_level: &str, file: &Option<String>) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightWhite)
        .debug(Color::BrightCyan)
        .info(Color::BrightGreen)
        .warn(Color::BrightYellow)
        .error(Color::BrightRed);
    let mut dispatch = Dispatch::new()
        .format(move |out, message, record| {
            // explicitly handle potentially malicious escape sequences
            let mut formatted_message = String::new();
            for c in message.to_string().chars() {
                match c {
                    '\n' => formatted_message.push_str("\\n"),
                    '\r' => formatted_message.push_str("\\r"),
                    '\u{0008}' => formatted_message.push_str("\\u{{0008}}"),
                    _ => formatted_message.push(c),
                }
            }

            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                colors.color(record.level()),
                record.target(),
                formatted_message
            ));
        })
        .level(LevelFilter::from_str(log_level).unwrap_or(LevelFilter::Info))
        .level_for("sqlx", LevelFilter::Warn)
        .chain(std::io::stdout());
    if let Some(file) = file {
        dispatch = dispatch.chain(fern::log_file(file)?);
    }
    Ok(dispatch.apply()?)
}

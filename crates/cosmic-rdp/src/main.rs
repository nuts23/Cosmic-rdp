mod app;
mod config;
mod i18n;

fn main() -> cosmic::iced::Result {
    // Initialize tracing / logger
    tracing_subscriber::fmt::init();

    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(480.0)
                .min_height(360.0),
        );

    // Starts the application's event loop
    cosmic::app::run::<app::AppModel>(settings, ())
}

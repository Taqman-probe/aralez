pub trait LoggerModule {
    fn new(level_str: &str, path: &Option<String>, config: Option<String>) -> Self;
    fn init(self);
}
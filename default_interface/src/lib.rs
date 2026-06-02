use std::any::Any;

pub struct ApplicationLogHandle {
    _inner: Option<Box<dyn Any + Send + Sync>>
}

impl ApplicationLogHandle {
    pub fn new<T: Send + Sync + 'static>(handle: T) -> Self {
        Self {
            _inner: Some(Box::new(handle)),
        }
    }

    pub fn empty() -> Self {
        Self { _inner: None }
    }
}

pub trait LoggerModule {
    fn new(level_str: &str, path: &Option<String>, config: Option<String>) -> Self;
    fn init(self) -> ApplicationLogHandle;
}
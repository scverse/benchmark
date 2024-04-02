pub(crate) trait RunConfig {
    fn repo(&self) -> &str;
    fn config_ref(&self) -> Option<&str>;
    fn run_on(&self) -> &[String];
}

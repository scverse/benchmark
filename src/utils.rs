pub(crate) trait PipeMap: tap::Pipe {
    fn pipe_map<O>(self, option: Option<O>, func: impl FnOnce(Self, O) -> Self) -> Self
    where
        Self: Sized,
        O: Sized,
    {
        if let Some(inner) = option {
            func(self, inner)
        } else {
            self
        }
    }

    fn pipe_map_ref<O>(
        &mut self,
        option: Option<O>,
        func: impl FnOnce(&mut Self, O) -> &mut Self,
    ) -> &mut Self
    where
        Self: Sized,
        O: Sized,
    {
        if let Some(inner) = option {
            func(self, inner)
        } else {
            self
        }
    }
}

impl<T: tap::Pipe> PipeMap for T {}

/// Get a systemd credential (see <https://systemd.io/CREDENTIALS/>).
pub(crate) fn get_credential(name: &str) -> anyhow::Result<secrecy::SecretString> {
    use libsystemd::credentials::CredentialsLoader;
    use std::io::{BufReader, Read};

    let loader = CredentialsLoader::open()?;
    let file = loader.get(name)?;
    let mut buffer = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut buffer)?;
    Ok(buffer.into())
}

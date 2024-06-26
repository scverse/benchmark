/// Get a systemd credential (see <https://systemd.io/CREDENTIALS/>).
#[cfg(target_os = "linux")]
pub(crate) fn get_credential(name: &str) -> anyhow::Result<secrecy::SecretString> {
    use libsystemd::credentials::CredentialsLoader;
    use std::io::{BufReader, Read};

    let loader = CredentialsLoader::open()?;
    let file = loader.get(name)?;
    let mut buffer = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut buffer)?;
    buffer.shrink_to_fit();
    Ok(buffer.into())
}

/// Fail to get a systemd credential as we’re not on Linux.
#[cfg(not(target_os = "linux"))]
pub(crate) fn get_credential(_name: &str) -> anyhow::Result<secrecy::SecretString> {
    Err(anyhow::anyhow!("No way to get credential on this OS"))
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
    use super::*;

    use secrecy::ExposeSecret;
    use std::{fs::File, io::Write};

    const TEST_SECRET: &str = "It's a Secret to Everybody";

    #[test]
    fn test_load_credential() {
        let tmp_dir = test_temp_dir::test_temp_dir!();
        let cred = tmp_dir.used_by(|p| {
            File::create(p.join("foo"))
                .unwrap()
                .write_all(TEST_SECRET.as_bytes())
                .unwrap();

            temp_env::with_var("CREDENTIALS_DIRECTORY", Some(p), || {
                get_credential("foo").unwrap()
            })
        });

        assert_eq!(cred.expose_secret(), TEST_SECRET);
    }
}

use crate::config::Config;

mod openrouter;
mod random;

pub enum LinkGenerator {
    Random,

    #[allow(dead_code)]
    OpenrouterLlama(String),

    // TODO come up with approach to do this case modular
    OpenrouterLlamaWithFallback(String),
}

impl LinkGenerator {
    pub async fn generate(&self, full_link: &str, bad_attempts: &[String]) -> Option<String> {
        match self {
            LinkGenerator::Random => Some(crate::link_generator::random::generate()),
            LinkGenerator::OpenrouterLlama(token) => {
                crate::link_generator::openrouter::generate(full_link, token, bad_attempts).await
            }
            LinkGenerator::OpenrouterLlamaWithFallback(token) => {
                if bad_attempts.len() == 2 {
                    log::info!(
                        "Too many attempts for AI generator for link {}, fallback random",
                        full_link
                    );
                    return Some(crate::link_generator::random::generate());
                }
                crate::link_generator::openrouter::generate(full_link, token, bad_attempts)
                    .await
                    .or_else(|| {
                        log::warn!(
                            "Cannot generate unique short link with openrouter for link {}",
                            full_link
                        );
                        Some(crate::link_generator::random::generate())
                    })
            }
        }
    }
    pub fn from_config(config: &Config) -> Self {
        match &config.openrouter_token {
            Some(token) => LinkGenerator::OpenrouterLlamaWithFallback(token.clone()),
            None => LinkGenerator::Random,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_link_generator() {
        use std::collections::HashSet;
        let link_generator = LinkGenerator::Random;
        let keys = ["key1", "key2", "key3"];
        let mut short_links = HashSet::<String>::new();
        for key in keys.iter() {
            short_links.insert(
                link_generator
                    .generate(key, &vec![])
                    .await
                    .expect("Cannot generate key"),
            );
        }
        assert!(short_links.len() == keys.len());
    }
}

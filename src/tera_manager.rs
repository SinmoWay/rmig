use std::collections::HashMap;
use tera::{Context, Tera};
use log::warn;
use crate::Error;

pub struct TeraManager {
    pub(crate) env: HashMap<String, String>,
}

impl TeraManager {
    pub fn new(env: HashMap<String, String>) -> Self {
        TeraManager { env }
    }

    pub fn apply(self, name: &str, value: &str) -> anyhow::Result<String, Error> {
        let mut ctx = Context::new();
        ctx = self.apply_context(ctx);
        let mut tera = Tera::default();
        tera.add_raw_template(name, value).map_err(|e| Error::ParseError(name.to_owned(), e.to_string()))?;
        tera.render(name, &ctx)
            .map_err(|e| {
                warn!("Error while parsing and resolving template {}. Context env: {:?}", &name, &ctx);
                Error::ParseError(name.to_owned(), format!("{:?}", e))
            })
    }

    fn apply_context(self, mut ctx: Context) -> Context {
        &self.env.iter().for_each(|kv| {
            ctx.insert(kv.0, kv.1)
        });
        ctx
    }
}

#[cfg(test)]
mod local_test {
    use std::collections::HashMap;
    use crate::tera_manager::TeraManager;

    #[test]
    fn test_tera_manager() -> anyhow::Result<()> {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("name"), String::from("WORLD"));
        let result = TeraManager::new(env).apply("hello", "SELECT {{ name }} FROM DUAL;")?;
        println!("Result: {}", &result);
        assert_eq!("SELECT WORLD FROM DUAL;", result.as_str());
        Ok(())
    }

    #[test]
    fn test_init_sql() -> anyhow::Result<()> {
        let value = include_str!("init/pg_init.sql");
        let mut result = TeraManager::new(HashMap::new()).apply("pg_init.sql", value)?.trim().to_owned();
        println!("Result: {}", &result);
        assert!(&result.starts_with("CREATE TABLE IF NOT EXISTS CHANGELOGS"));

        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("SCHEMA_ADMIN"), String::from("WORLD"));
        result = TeraManager::new(env).apply("pg_init.sql", value)?.trim().to_owned();
        println!("Result: {}", &result);
        assert!(&result.starts_with("CREATE TABLE IF NOT EXISTS WORLD.CHANGELOGS"));
        Ok(())
    }
}
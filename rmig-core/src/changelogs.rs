use glob::glob;
use serde::{Serialize, Deserialize};
use std::collections::{VecDeque, HashMap};
use log::{debug, trace};
use crate::tera_manager::TeraManager;
use std::path::PathBuf;
use std::str::FromStr;
use crate::driver::Driver;
use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Changelogs {
    pub changelogs: Vec<Changelog>,
    properties: HashMap<String, String>,
}

// TODO: Delete author.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Changelog {
    /// Migration name
    pub name: String,
    /// Migration
    #[serde(skip_serializing, skip_deserializing)]
    pub order: i16,
    /// Directory. Support wildcard
    pub directory: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub _directory: Directory,
    /// Author name
    pub author: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Directory {
    pub name: String,
    pub migration_list: VecDeque<Migration>,
    pub _directory: Option<Box<VecDeque<Directory>>>,
}

impl Directory {
    pub fn new(name: String) -> Self {
        Directory { name, migration_list: VecDeque::new(), _directory: None }
    }
}

impl FromStr for Directory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Directory::new(s.to_owned()))
    }
}

impl Default for Directory {
    fn default() -> Self {
        Directory { name: ".".to_owned(), migration_list: Default::default(), _directory: None }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Migration {
    /// Migration name
    pub name: String,
    /// Hash MD5
    pub hash: String,
    /// Separator for split query
    separator: String,
    /// Execution order
    pub order: i64,
    /// Query's
    pub query_list: VecDeque<Query>,
    /// Global migration options
    pub options: Option<QueryOptions>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Query {
    pub query: String,
    pub opts: QueryOptions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryOptions {
    pub has_run: Option<bool>,
    pub run_always: Option<bool>,
    pub global: Option<bool>,
}

impl QueryOptions {
    pub fn is_global(&self) -> bool {
        return self.global.as_ref().is_some();
    }
}

impl Default for QueryOptions {
    fn default() -> Self {
        QueryOptions {
            has_run: Some(false),
            run_always: Some(false),
            global: Some(false),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChangelogReader<'a> {
    separator: &'a str,
    params: Option<HashMap<String, String>>,
}

impl<'a> Default for ChangelogReader<'a> {
    fn default() -> Self {
        ChangelogReader {
            separator: "-->",
            params: None,
        }
    }
}

impl<'a> ChangelogReader<'a> {
    pub fn new(separator: &'a str) -> Self {
        ChangelogReader {
            separator,
            params: None,
        }
    }

    pub fn read_changelog_with_env(mut self, yaml_file: String, env: Option<HashMap<String, String>>) -> anyhow::Result<Changelogs, Error> {
        self.params = env;
        let mut yaml = std::fs::read_to_string(yaml_file.as_str()).map_err(|e| Error::IOError(e.to_string()))?;


        if self.params.as_ref().is_some() {
            yaml = TeraManager::new(self.params.as_ref().unwrap().clone()).apply("changelogs.yml", yaml.as_str())?;
        }

        let mut changelogs: Changelogs = serde_yaml::from_str(yaml.as_str())
            .map_err(|e| Error::ParseError(yaml_file.to_owned(), e.to_string()))?;

        if self.params.as_ref().is_some() {
            let mut x = self.params.as_ref().unwrap().clone();
            x.extend(changelogs.clone().properties);
            self.params = Some(x);
        }
        for c in changelogs.changelogs.iter_mut() {
            let dir = self.read_directory(Directory::from_str(&c.directory)?)?;
            c._directory = dir;
        };

        Ok(changelogs)
    }

    pub fn read_directory(&self, mut dir: Directory) -> anyhow::Result<Directory, Error> {
        let paths = glob(&dir.name)
            .map_err(|e| Error::IOError(e.msg.to_owned()))?;
        for path in paths {
            let path_buf = path.expect("Error while getting file path.");
            debug!("Including path's: {:?}", path_buf);
            if !path_buf.is_dir() {
                let migration = self.clone().read_migration(&path_buf)?;
                // FIXME: Impl normal by order
                dir.migration_list.push_back(migration)
            } else {
                let x = path_buf.to_str().expect("File path is not readable.");
                let sub_directory = self.read_directory(Directory::from_str(format!("{}{}", x, "/*").as_str())?)?;

                fn create_directory(dir: Directory) -> Option<Box<VecDeque<Directory>>> {
                    let mut deque = VecDeque::<Directory>::new();
                    deque.push_back(dir);
                    return Some(Box::new(deque));
                }

                //Посмотреть что тут за пиздец
                dir._directory = Some(dir._directory.as_mut().map(|d| {
                    d.push_back(sub_directory.clone());
                    d
                }).unwrap_or(&mut create_directory(sub_directory.clone()).unwrap()).to_owned());
            }
        }
        Ok(dir)
    }

    pub fn read_migration(self, path: &PathBuf) -> anyhow::Result<Migration, Error> {
        let mut sql = std::fs::read_to_string(path).map_err(|e| Error::IOError(e.to_string()))?;

        let name = path
            .file_name()
            .expect("Error while read migration. Expected filename for file.")
            .to_str()
            .expect("Error while read migration. Filename is not readable.")
            .to_string();

        if self.params.as_ref().is_some() {
            sql = TeraManager::new(self.params.as_ref().unwrap().clone()).apply(&*name, sql.as_str())?;
        }

        let hash = format!("{:x}", md5::compute(&sql));
        let name_separate = name.trim().split('.').collect::<Vec<&str>>();
        // TODO: Add order to version or remove.
        let mut order = 0i64;
        // Extension .sql/.any
        if name_separate.len() > 1 {
            order = name_separate[0]
                .parse()
                .map_err(|_e| Error::ParseFileError("File name is not contains order. Please use format order.filename.any extension. For example: 1.init.sql".to_owned()))?;
        } else {
            return Err(Error::ParseError(name, "Error while read migration. Extension required.".to_string()));
        }

        // Collections separating lines
        let query = sql
            .as_str()
            .split(&self.separator)
            .collect::<Vec<&str>>();

        let mut querys = VecDeque::<Query>::with_capacity(query.len());

        debug!("Reading migration by file: {}", &name);
        let mut global_options_for_query = None::<QueryOptions>;
        // Надо дописать нормально обработку options.
        for q in query {
            let mut qu = self.clone().read_query(q)?;
            if qu.opts.is_global() {
                global_options_for_query.insert(qu.opts.clone());
            }

            if global_options_for_query.as_ref().is_some() {
                qu.opts = global_options_for_query.as_ref().unwrap().clone();
            }
            querys.push_back(qu)
        }

        Ok(Migration {
            name: path.as_path().to_str().map(|m| m.to_owned()).unwrap(),
            hash,
            separator: self.separator.to_string(),
            order,
            query_list: querys,
            options: None,
        })
    }

    pub fn read_options(&self, text: &str) -> anyhow::Result<Option<QueryOptions>, Error> {
        let mut opts: Option<QueryOptions> = Option::None;
        if text.starts_with("--rmig--") {
            let lines = text.lines().collect::<Vec<&str>>();

            let sum = lines.len() as u8;
            if sum > 1u8 {
                let opts_str = lines[0].replace("--rmig--", "");
                // TODO: Посмотреть по коду что-то тут не так.
                opts = Some(serde_json::from_str(opts_str.as_str())
                    .map_err(|e| Error::ParseFileError(format!("Options in query by {} is not parse.\n Serialize error: {:?}", text, e)))?);
            }
        }
        Ok(opts)
    }

    pub fn read_query(self, text: &str) -> anyhow::Result<Query, Error> {
        let query = text.trim_start().trim_end().to_string();
        let opts: Option<QueryOptions> = self.read_options(&*query)?;

        if opts.as_ref().is_some() {
            let without_option_line = query.lines().skip(1).collect::<Vec<&str>>().join("\n");
            Ok(Query {
                query: without_option_line,
                opts: opts.unwrap_or_default(),
            })
        } else {
            Ok(Query {
                query,
                opts: opts.unwrap_or_default(),
            })
        }
    }
}

pub struct ChangelogRunner<'a> {
    pub changelog: Changelogs,
    pub datasources: Vec<&'a Box<dyn Driver>>,
    pub properties: Option<HashMap<String, String>>,
}

impl<'a> ChangelogRunner<'a> {
    pub fn new_from_file(changelog_path: String,
                         datasources: Vec<&'a Box<dyn Driver>>,
                         properties: Option<HashMap<String, String>>) -> Self {
        let changelog_reader = properties.as_ref()
            .and_then(|p| p.get("query_separator"))
            .map(|path| ChangelogReader::new(path.as_str()))
            .unwrap_or_default();
        ChangelogRunner {
            changelog: changelog_reader
                .read_changelog_with_env(changelog_path.to_owned(), properties.clone())
                .unwrap_or_else(|e| panic!("Error while reading changelog with name {}", &*changelog_path)),
            datasources,
            properties,
        }
    }

    pub fn filter_by_stage(mut self, stages: Vec<String>) -> Self {
        if !stages.is_empty() {
            debug!("Starting filtering changelog, current size: {}", &self.changelog.changelogs.len());
            self.changelog.changelogs = self.changelog.changelogs.into_iter().filter(|n| {
                trace!("Filtering changelog ");
                stages.contains(&n.name)
            }).collect();
            debug!("After filtering size: {}", &self.changelog.changelogs.len());
        }
        self
    }
}

/// TODO: Write tests
#[cfg(test)]
mod local_test {
    use crate::changelogs::{QueryOptions, ChangelogRunner};
    use std::collections::HashMap;

    #[test]
    pub fn test_md5() -> anyhow::Result<()> {
        let md5 = format!("{:x}", md5::compute("hello world"));
        assert_eq!("5eb63bbbe01eeed093cb22bb8f5acdc3", md5);
        Ok(())
    }
}
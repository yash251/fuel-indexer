use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub namespace: String,
    pub abi: Option<String>,
    pub identifier: String,
    pub graphql_schema: String,
    pub module: Module,
    pub metrics: Option<bool>,
    pub contract_id: Option<String>,
    pub start_block: Option<u64>,
}

type ManifestResult<T> = Result<T, ManifestError>;

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Compiler error: {0:#?}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Native module bytes not supported.")]
    NativeModuleError,
    #[error("File IO error: {0:?}.")]
    FileError(#[from] std::io::Error),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Module {
    Wasm(String),
    Native,
}

impl Module {
    pub fn path(&self) -> String {
        match self {
            Self::Wasm(o) => o.clone(),
            Self::Native => unimplemented!(),
        }
    }
}

impl Manifest {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> ManifestResult<Self> {
        let manifest: Manifest = serde_yaml::from_str(content)?;
        Ok(manifest)
    }

    pub fn from_file(path: &Path) -> ManifestResult<Self> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Self::from_str(&content)
    }

    pub fn from_slice(s: &[u8]) -> ManifestResult<Self> {
        Ok(serde_yaml::from_slice(s)?)
    }

    pub fn to_bytes(&self) -> ManifestResult<Vec<u8>> {
        Ok(serde_yaml::to_string(&self)?.as_bytes().to_vec())
    }

    pub fn graphql_schema(&self) -> ManifestResult<String> {
        let mut file = File::open(&self.graphql_schema)?;
        let mut schema = String::new();
        file.read_to_string(&mut schema)?;

        Ok(schema)
    }

    pub fn uid(&self) -> String {
        format!("{}.{}", &self.namespace, &self.identifier)
    }

    pub fn is_native(&self) -> bool {
        match &self.module {
            Module::Native => true,
            Module::Wasm(_o) => false,
        }
    }

    pub fn module_bytes(&self) -> ManifestResult<Vec<u8>> {
        match &self.module {
            Module::Wasm(p) => {
                let mut bytes = Vec::<u8>::new();
                let mut file = File::open(p)?;
                file.read_to_end(&mut bytes)?;

                Ok(bytes)
            }
            Module::Native => unimplemented!(),
        }
    }

    pub fn write_to(&self, path: &PathBuf) -> ManifestResult<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes()?)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub trigger: String,
    pub payload: String,
}

use crate::code::Language;
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct CommandChainParams {
    pub commands: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct GitParams {
    pub operation: String,
    pub args: Vec<String>,
    pub description: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct CodeGenParams {
    pub language: Language,
    pub description: String,
    pub path: Option<String>,
}


pub mod read_file;
pub mod generate_image;
pub mod image_generator_parameter;

use std::{collections::HashMap, fmt};
use aws_sdk_bedrockruntime::types::{ToolResultBlock, ToolResultContentBlock, ToolResultStatus};
use anyhow::Result;
use aws_smithy_types::Document;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::result::Result::Ok;


pub fn create_tool_result_block(id: &str, content: &str, status: ToolResultStatus ) -> Result<ToolResultBlock> {
    let tool_result = ToolResultBlock::builder()
        .tool_use_id(id.to_owned())
        .content(ToolResultContentBlock::Text(content.to_owned()))
        .status(status)
        .build()?;
    Ok(tool_result)
}


pub trait ToDocument {
    fn to_document(&self) -> Document;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolJsonSchema {
    r#type: String,
    pub properties: HashMap<String, Property>,
    pub required: Vec<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Property {
    pub r#type: PropertyType,
    pub description: String
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PropertyType {
    String,
    Object,
    Number,
    Array,
    Boolean,
    Null
}
impl fmt::Display for PropertyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ToDocument for Property {
    fn to_document(&self) -> Document {
        Document::Object(HashMap::from([
            (
                "type".to_owned(),
                Document::String(self.r#type.to_string().to_lowercase().to_owned())
            ),
            (
                "description".to_owned(),
                Document::String(self.description.to_owned()),
            ),
        ]))
    }
}


impl ToolJsonSchema {
    pub fn new(json: Value) -> Result<Self> {
        let schema: ToolJsonSchema  = serde_json::from_value(json)?;
        Ok(schema)
    }
}

impl ToDocument for ToolJsonSchema {
    fn to_document(&self) -> Document {
        let r#type = Document::String(self.r#type.to_owned());
        let properties: HashMap<String, Document> = self.properties.to_owned()
            .iter().map(|(k, v)| (k.to_owned(), v.to_document())).collect();
        let required: Vec<Document> =self.required.to_owned().iter().map(|r| Document::String(r.to_owned())).collect();

        Document::Object(HashMap::<String, Document>::from([
            ("type".to_owned(), r#type),
            ("properties".to_owned(), Document::Object(properties)),
            ("required".to_owned(), Document::Array(required)),
        ]))
    }
}

impl ToDocument for Value {
    fn to_document(&self) -> Document {
        if let Some(string) = self.as_str() {
            if string == "null" {
                return Document::Null;
            } else {
                return Document::String(string.to_owned());
            }
        }

        if let Some(bool) = self.as_bool() {
            return Document::Bool(bool)
        }

        if let Some(f64) = self.as_f64() {
            return Document::Number(aws_smithy_types::Number::Float(f64));
        }

        if let Some(array) = self.as_array() {
            let mut doc_array: Vec<Document> = vec![];
            for item in array {
                doc_array.push(item.to_document())
            }
            return Document::Array(doc_array);
        }

        if let Some(object) = self.as_object() {
            let mut doc_map: HashMap<String, Document> = HashMap::new();
            for (key, value) in object.into_iter() {
                doc_map.insert(key.to_owned(), value.to_document());
            };
            return Document::Object(doc_map);
        }

        return Document::Null;
    }
}

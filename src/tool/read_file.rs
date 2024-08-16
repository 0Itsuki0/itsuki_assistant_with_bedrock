use super::{create_tool_result_block, ToDocument, ToolJsonSchema};

use core::str;
use std::{borrow::Borrow, fs, path::PathBuf};
use anyhow::{bail,  Result};
use aws_sdk_bedrockruntime::types::{DocumentBlock, DocumentFormat, DocumentSource, ToolResultBlock, ToolResultContentBlock, ToolResultStatus};
use aws_smithy_types::{Blob, Document};
use serde_json::json;


// READ_FILE tool
pub const READ_FILE_NAME: &str = "READ_FILE";
pub const READ_FILE_DESCRIPTION: &str = "Read the contents of a file at the specified path. Use this when you need to examine the contents of an existing file.";
pub fn read_file_schema() -> Result<Document> {
    let json_schema = json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "The path of the file to read."
            },
        },
        "required": ["path"],
    });
    let document = ToolJsonSchema::new(json_schema)?.to_document();
    Ok(document)
}

pub fn read_file(id: &str, input: &Document) -> Result<ToolResultBlock> {
    let input_object = match input.as_object() {
        Some(object) => object,
        None => {
            return create_tool_result_block(id, "failed to convert input to object", ToolResultStatus::Error)
        },
    };

    let path = match input_object.get("path") {
        Some(object) => {
            match object.as_string()  {
                Some(s) => s,
                None => {
                    return create_tool_result_block(id, "path to read file from is not a string.", ToolResultStatus::Error)
                },
            }
        },
        None => {
            return create_tool_result_block(id, "path to read file from is not provided", ToolResultStatus::Error)
        },
    };

    let path = PathBuf::from(path);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return create_tool_result_block(id, &err.to_string(), ToolResultStatus::Error)
        },
    };

    if let Some(extension) = path.extension() {
        let str = extension.to_str().unwrap_or("");
        if let Ok(format) = get_format_from_extension(str) {
            let document = DocumentBlock::builder()
                .name("file_read")
                .format(format)
                .source(DocumentSource::Bytes(Blob::new(bytes)))
                .build()?;

            let tool_result = ToolResultBlock::builder()
                .tool_use_id(id.to_owned())
                .content(ToolResultContentBlock::Text("File read.".to_owned()))
                .content(ToolResultContentBlock::Document(document))
                .status(ToolResultStatus::Success)
                .build()?;

            return Ok(tool_result)
        }
    }

    let string = match str::from_utf8(&bytes) {
        Ok(string) => string,
        Err(err) => {
            return create_tool_result_block(id, &err.to_string(), ToolResultStatus::Error)
        },
    };
    let content = format!("File read with Content: {}", string);
    return create_tool_result_block(id, &content, ToolResultStatus::Success)
}


fn get_format_from_extension(extension: &str) -> Result<DocumentFormat> {
    match extension.to_lowercase().borrow() {
        "pdf" => Ok(DocumentFormat::Pdf),
        "csv" => Ok(DocumentFormat::Csv),
        "doc" => Ok(DocumentFormat::Doc),
        "docx" => Ok(DocumentFormat::Docx),
        "html" => Ok(DocumentFormat::Html),
        "md" => Ok(DocumentFormat::Md),
        "txt" => Ok(DocumentFormat::Txt),
        "xls" => Ok(DocumentFormat::Xls),
        "xlsx" => Ok(DocumentFormat::Xlsx),
        _ => bail!(format!("No format available for extension: {extension}"))
    }
}
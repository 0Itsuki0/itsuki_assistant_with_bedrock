use super::{create_tool_result_block, ToDocument, ToolJsonSchema};

use std::path::Path;
use std::fs::{self};

use anyhow::Result;
use aws_sdk_bedrockruntime::types::{ToolResultBlock, ToolResultStatus};
use aws_smithy_types::Document;
use serde_json::json;
use base64::prelude::*;


// GENERATE_IMAGE tool
pub const GENERATE_IMAGE_NAME: &str = "GENERATE_IMAGE";
pub const GENERATE_IMAGE_DESCRIPTION: &str = "Generate an image based on user's prompt.";

pub const DEFAULT_HEIGHT: u128 = 512;
pub const DEFAULT_WIDTH: u128 = 512;

pub fn generate_image_schema() -> Result<Document> {
    let json_schema = json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "Description for the image to generate. Required."
            },
            "path": {
                "type": "string",
                "description": "The path of the folder where the generated image should be saved. Required.Default to the current working directory."
            },
            "numberOfImages": {
                "type": "number",
                "description": "The number of images to generate. Optional. The default value is 1."
            },
            "quality": {
                "type": "string",
                "description": "The quality of the image to generate. Optional. Possible value: standard, premium. The default value is standard."
            },
            "height": {
                "type": "number",
                "description": "The height of the image in pixels. Optional. The default value is ".to_owned() + &DEFAULT_HEIGHT.to_string() + " .",
            },
            "width": {
                "type": "number",
                "description": "The width of the image in pixels. Optional. The default value is ".to_owned() + &DEFAULT_WIDTH.to_string() + " .",
            },

        },
        "required": ["prompt", "path"],
    });
    let document = ToolJsonSchema::new(json_schema)?.to_document();
    Ok(document)
}

pub fn save_generated_image(id: &str, input: &Document, images: Vec<String>) -> Result<ToolResultBlock> {
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
                    return create_tool_result_block(id, "path to save the image is not a string.", ToolResultStatus::Error)
                },
            }
        },
        None => {
            return create_tool_result_block(id, "path to save the image is not provided", ToolResultStatus::Error)
        },
    };

    let mut path = Path::new(path);
    if path.extension().is_some() {
        path = match path.parent() {
            Some(path) => path,
            None => {
                return create_tool_result_block(id, "cannot get a path to save the image.", ToolResultStatus::Error)
            }
        }
    }

    if !path.to_str().unwrap_or("").is_empty() {
        match fs::create_dir_all(path) {
            Ok(_) => {},
            Err(err) => {
                return create_tool_result_block(id, &err.to_string(), ToolResultStatus::Error)
            }
        };
    }

    match open::that_detached(path) {
        Ok(_) => {},
        Err(_) => {},
    };

    for (index, image_string) in images.into_iter().enumerate() {
        let image_name = Path::new(&format!("{}-{}.png", id, index)).to_owned();
        let image_path = path.join(image_name);

        let bytes = BASE64_STANDARD.decode(image_string)?;
        let image = match image::load_from_memory(&bytes) {
            Ok(image) => image,
            Err(err) => {
                return create_tool_result_block(id, &err.to_string(), ToolResultStatus::Error)
            },
        };

        match image.save(&image_path) {
            Ok(_) => {},
            Err(err) => {
                return create_tool_result_block(id, &err.to_string(), ToolResultStatus::Error)
            },
        };

        match open::that_detached(image_path) {
            Ok(_) => {},
            Err(_) => {},
        };
    }
    return create_tool_result_block(id, "Image generated and saved.", ToolResultStatus::Success)
}

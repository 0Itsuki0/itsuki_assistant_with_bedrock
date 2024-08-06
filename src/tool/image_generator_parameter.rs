use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGeneratorParameter {
    pub task_type: TaskType,
    pub text_to_image_params: TextToImageParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_generation_config: Option<ImageGenerationConfig>
}

impl ImageGeneratorParameter {
    pub fn new_generate_image_params(prompt: &str, image_generation_config: Option<ImageGenerationConfig>) -> Self {
        Self {
            task_type: TaskType::TextImage,
            text_to_image_params: TextToImageParams { text: prompt.to_owned() },
            image_generation_config,
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskType {
    TextImage,
    Inpainting,
    Outpainting,
    ImageVariation
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextToImageParams {
    pub text: String
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<IamgeQuality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u128>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IamgeQuality {
    Standard,
    Premium
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGeneratorResponse {
    pub images: Vec<String>,
}

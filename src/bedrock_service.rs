
use core::str;
use std::env;
use anyhow::{bail, Context, Result};
use aws_sdk_bedrockruntime::types::{ContentBlockDelta, ConverseStreamOutput, StopReason};
use aws_smithy_types::{Blob, Document};
use aws_sdk_bedrockruntime::Client;
use aws_sdk_bedrockruntime::types::{ContentBlock, Message, SystemContentBlock, Tool, ToolConfiguration, ToolInputSchema, ToolSpecification, ConversationRole::{User, Assistant}, ToolResultBlock, ToolResultStatus, ToolUseBlock};
use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use serde_json::Value;

use crate::tool::{create_tool_result_block, ToDocument};
use crate::tool::read_file::{read_file, read_file_schema, READ_FILE_DESCRIPTION, READ_FILE_NAME};
use crate::tool::image_generator_parameter::{IamgeQuality, ImageGenerationConfig, ImageGeneratorParameter, ImageGeneratorResponse};
use crate::tool::generate_image::{generate_image_schema, save_generated_image, DEFAULT_HEIGHT, DEFAULT_WIDTH, GENERATE_IMAGE_DESCRIPTION, GENERATE_IMAGE_NAME};
use crate::tool::run_python::{run_python, run_python_schema, RUN_PYTHON_DESCRIPTION, RUN_PYTHON_NAME};

use crate::terminal_service::TerminalService;
use crate::model_constants::{CHAT_MODEL_ID, CHAT_MODEL_KEY, IMAGE_MODEL_ID, IMAGE_MODEL_KEY};

fn get_system_prompt() -> String {
    return format!("
        You are Claude, an AI assistant and an exceptional designer and software engineer with vast knowledge across multiple programming languages, frameworks, and best practices.
        You strictly follow the following rules.

        Your capabilities include:
        1. Chat
        2. Answer User's questions on files
        3. Create/Generate new Image based on user's prompt
        4. Perform data analysis/math using python and run Python code to solve the user's task.

        You are familiar with the following python libraries.
        - pandas
        - numpy
        - matplotlib
        - seaborn
        - scikit-learn
        - diagrams, etc.

        Choose the tool that BEST FIT the task.
        For example, when asked for a graph of y=x, you should use {RUN_PYTHON_NAME} instead of {GENERATE_IMAGE_NAME}.

        When asked to create/generate a new image:
        - Use {GENERATE_IMAGE_NAME} tool to generate an image.
        - Verify for the file path to save the image. If not provided, ask for it.

        When asked to perform data analysis or math:
        - If you need the file content to perform analysis on, use the {READ_FILE_NAME} tool first
        - Use {RUN_PYTHON_NAME} tool to run python code for analysis

        You can read files from local disk using {READ_FILE_NAME} tool. Use these capabilities when:
        - The user asks questions regarding to existing files
        - You need to examine the contents of an existing file

        To use the tools provided,
        - Strictly apply the provided tool specification.
        - Never guess or make up information. If not enough information provided, ask for it.
        - Use the tool ONLY if you have all the required data.
        - When generating code, don NOT write the code that contains code to read the data or file. Your code will be run in a seperate sandbox.
        - Add constructive comments when writing code.
    ");
}



#[derive(Debug)]
pub struct BedrockService {
    bedrock_client: Client,
    chat_model_id: String,
    image_model_id: String,
    system_prmopt: SystemContentBlock,
    conversation: Vec<Message>,
    tool_config: ToolConfiguration,
    terminal: TerminalService
}

// public impl
impl BedrockService {
    pub fn new(client: &Client) -> Result<Self> {
        let system_prmopt = SystemContentBlock::Text(get_system_prompt());

        let generate_image_tool = Tool::ToolSpec(
            ToolSpecification::builder()
                .name(GENERATE_IMAGE_NAME)
                .description(GENERATE_IMAGE_DESCRIPTION)
                .input_schema(ToolInputSchema::Json(generate_image_schema()?))
                .build()?
        );

        let read_file_tool = Tool::ToolSpec(
            ToolSpecification::builder()
                .name(READ_FILE_NAME)
                .description(READ_FILE_DESCRIPTION)
                .input_schema(ToolInputSchema::Json(read_file_schema()?))
                .build()?
        );

        let run_python_tool = Tool::ToolSpec(
            ToolSpecification::builder()
                .name(RUN_PYTHON_NAME)
                .description(RUN_PYTHON_DESCRIPTION)
                .input_schema(ToolInputSchema::Json(run_python_schema()?))
                .build()?
        );

        let tool_configuration = ToolConfiguration::builder()
            .set_tools(Some(vec![read_file_tool, generate_image_tool, run_python_tool]))
            // .tool_choice(ToolChoice::Tool(SpecificToolChoice::builder().name(CREATE_FILE_NAME).build()?))
            .build()?;

        Ok(
            Self {
                bedrock_client: client.to_owned(),
                chat_model_id: env::var(CHAT_MODEL_KEY).unwrap_or(CHAT_MODEL_ID.to_owned()),
                image_model_id: env::var(IMAGE_MODEL_KEY).unwrap_or(IMAGE_MODEL_ID.to_owned()),
                system_prmopt,
                conversation: vec![],
                tool_config: tool_configuration,
                terminal: TerminalService::new()
            }
        )
    }

    // non streaming
    pub async fn run(&mut self, input: &str) -> Result<()> {

        self.append_user_message(input)?;

        let response = match self.send().await {
            Ok(response) => response,
            Err(err) => {
                self.terminal.clear_line()?;
                self.terminal.log_error(&err.root_cause().to_string())?;
                return Ok(());
            },
        };
        self.terminal.clear_line()?;
        match self.process_output(response).await {
            Ok(_) => {},
            Err(err) => {
                self.terminal.clear_line()?;
                self.terminal.log_error(&err.root_cause().to_string())?;
                return Ok(());
            },
        };
        Ok(())
    }


    async fn send(&mut self) -> Result<ConverseOutput> {

        let builder = self.bedrock_client
            .converse()
            .model_id(&self.chat_model_id)
            .system(self.system_prmopt.clone())
            .set_messages(Some(self.conversation.clone()))
            .tool_config(self.tool_config.clone());


        let response = builder
            .send()
            .await?;
        // println!("response.stop_reason: {:?}", response.stop_reason);
        Ok(response)
    }

    async fn process_output(&mut self, output: ConverseOutput) -> Result<()> {
        let output = output.output().context("Error getting output")?;
        let message = match output.as_message() {
            Ok(message) => message,
            Err(_) => {
                bail!("Output is not a message")
            },
        };
        self.conversation.push(message.clone());

        let contents = message.content();
        // println!("contents count: {}", contents.len());
        let mut tool_results: Vec<ContentBlock> = vec![];

        for content in contents {
            match content {
                ContentBlock::Text(text_content) => {
                    // println!("\x1b[0;90mThe model's response:\x1b[0m\n{text_content}");
                    self.terminal.log_ai(text_content)?;
                },
                ContentBlock::ToolUse(tool_use) => {
                    // println!("tool_use: {:?}", tool_use);
                    let name = tool_use.name();
                    let input = tool_use.input();
                    self.terminal.log_tool(name, input)?;
                    let result = self.use_tool(tool_use).await?;
                    tool_results.push(ContentBlock::ToolResult(result))
                },
                _ => {
                    break
                },
            }
        }

        if !tool_results.is_empty() {
            let tool_results_messgae = Message::builder()
                .role(User)
                .set_content(Some(tool_results))
                .build()?;
            self.conversation.push(tool_results_messgae);

            let tool_response = self.send().await?;
            let tool_response_output = tool_response.output().context("Error getting output")?;
            // println!("tool_response_output: {:?}", tool_response_output);
            let tool_response_message = match tool_response_output.as_message() {
                Ok(message) => message,
                Err(_) => {
                    bail!("Output is not a message")
                },
            };
            self.conversation.push(tool_response_message.clone());
            let tool_response_contents = tool_response_message.content();

            for tool_response_content in tool_response_contents {
                match tool_response_content {
                    ContentBlock::Text(text_content) => {
                        self.terminal.log_ai(text_content)?;
                        // println!("\x1b[0;90mThe model's response:\x1b[0m\n{text_content}");
                    },
                    _ => {
                        break
                    },
                }
            }
        }

        Ok(())
    }


    pub async fn run_stream(&mut self, input: &str) -> Result<()> {

        self.append_user_message(input)?;

        let response = match self.send_stream().await {
            Ok(response) => response,
            Err(err) => {
                self.terminal.clear_line()?;
                self.terminal.log_error(&err.root_cause().to_string())?;
                return Ok(());
            },
        };
        self.terminal.clear_line()?;
        match self.process_output_stream(response).await {
            Ok(_) => {},
            Err(err) => {
                self.terminal.clear_line()?;
                self.terminal.log_error(&err.root_cause().to_string())?;
                return Ok(());
            },
        };

        Ok(())
    }

    async fn send_stream(&mut self) -> Result<aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamOutput> {

        let builder = self.bedrock_client
            .converse_stream()
            .model_id(&self.chat_model_id)
            .system(self.system_prmopt.clone())
            .set_messages(Some(self.conversation.clone()))
            .tool_config(self.tool_config.clone());


        let response = builder
            .send()
            .await?;
        // println!("response.stop_reason: {:?}", response.stop_reason);
        Ok(response)
    }

    async fn process_output_stream(&mut self, output: aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamOutput) -> Result<()> {
        let mut stream = output.stream;
        let mut tool_input = "".to_owned();
        let mut tool_id = "".to_owned();
        let mut tool_name = "".to_owned();
        let mut assistant_message = "".to_owned();

        let mut tool_results: Vec<ContentBlock> = vec![];

        loop {
            let token = stream.recv().await?;
            match token {
                Some(output) => {
                    // self.process_stream_token(output)?
                    // println!("token: {:?}", output);
                    match output {
                        ConverseStreamOutput::ContentBlockDelta(event) => {
                            let delta = event.delta.context("delta in event not found")?;
                            match delta {
                                ContentBlockDelta::Text(text) => {
                                    self.terminal.log_ai_inline(&text)?;
                                    assistant_message = format!("{}{}", assistant_message, text)
                                },
                                ContentBlockDelta::ToolUse(tool) => {
                                    self.terminal.log_info_inline(&tool.input())?;
                                    tool_input = format!("{}{}", tool_input, tool.input())
                                },
                                _ => {
                                    continue;
                                },
                            }

                        },
                        ConverseStreamOutput::ContentBlockStart(event) => {
                            match event.start {
                                Some(start) => {
                                    match start {
                                        aws_sdk_bedrockruntime::types::ContentBlockStart::ToolUse(tool_use) => {
                                            tool_id = tool_use.tool_use_id;
                                            tool_name = tool_use.name;
                                            self.terminal.log_info(&format!("\n\rTool used: {tool_name}"))?;
                                            self.terminal.log_info_inline("\rTool Input: ")?;
                                        },
                                        _ => {
                                            continue;
                                        },
                                    }
                                },
                                None => {
                                    continue;
                                },
                            }
                        },
                        ConverseStreamOutput::MessageStart(_) => {
                            self.terminal.log_info("AI:\r")?;
                        }
                        ConverseStreamOutput::MessageStop(event) => {
                            self.terminal.log_info("\r")?;

                            if event.stop_reason == StopReason::ToolUse {

                                let tool_input = match serde_json::from_str::<Value>(&tool_input)  {
                                    Ok(value) => value.to_document(),
                                    Err(_) => Document::String(tool_input.clone()),
                                };

                                let tool_use_block = ToolUseBlock::builder()
                                    .name(tool_name.clone())
                                    .input(tool_input)
                                    .tool_use_id(tool_id.clone())
                                    .build()?;

                                let mut message_buider = Message::builder()
                                    .role(Assistant);

                                if !assistant_message.is_empty() {
                                    message_buider = message_buider.content(ContentBlock::Text(assistant_message.clone()))
                                }
                                message_buider = message_buider
                                    .content(ContentBlock::ToolUse(tool_use_block.clone()));

                                let message = message_buider.build()?;

                                self.conversation.push(message.clone());

                                let result = self.use_tool(&tool_use_block).await?;
                                tool_results.push(ContentBlock::ToolResult(result))

                            } else {
                                let message = Message::builder()
                                    .role(Assistant)
                                    .content(ContentBlock::Text(assistant_message.clone()))
                                    .build()?;
                                self.conversation.push(message.clone());
                            }

                            tool_id = "".to_string();
                            tool_input = "".to_string();
                            tool_name = "".to_string();
                            assistant_message = "".to_string();
                        }
                        _ => {
                            continue;
                        },
                    };
                },
                None => break,
            }
        }

        if !tool_results.is_empty() {
            let tool_results_messgae = Message::builder()
                .role(User)
                .set_content(Some(tool_results.clone()))
                .build()?;
            self.conversation.push(tool_results_messgae);

            let tool_response = self.send_stream().await?;
            let mut tool_stream = tool_response.stream;
            let mut tool_assistant_message = "".to_owned();

            loop {
                let token = tool_stream.recv().await?;
                match token {
                    Some(output) => {
                        match output {
                            ConverseStreamOutput::ContentBlockDelta(event) => {
                                let delta = event.delta.context("delta in event not found")?;
                                match delta {
                                    ContentBlockDelta::Text(text) => {
                                        self.terminal.log_ai_inline(&text)?;
                                        tool_assistant_message = format!("{}{}", tool_assistant_message, text)
                                    },
                                    _ => {
                                        continue;
                                    },
                                }

                            },
                            ConverseStreamOutput::MessageStart(_) => {
                                self.terminal.log_info("AI:\r")?;
                            }
                            ConverseStreamOutput::MessageStop(event) => {
                                self.terminal.log_info("\r")?;
                                if event.stop_reason == StopReason::EndTurn {
                                    let message = Message::builder()
                                        .role(Assistant)
                                        .content(ContentBlock::Text(tool_assistant_message.clone()))
                                        .build()?;
                                    self.conversation.push(message.clone());
                                }
                                tool_assistant_message = "".to_string();
                            }
                            _ => {
                                continue;
                            },
                        };
                    },
                    None => break,
                }
            }
        }

        Ok(())
    }

    fn append_user_message(&mut self, input: &str) -> Result<()> {
        let message = Message::builder()
            .role(User)
            .content(ContentBlock::Text(input.to_owned()))
            .build()?;

        self.conversation.push(message);
        Ok(())
    }


    async fn use_tool(&mut self, tool_use: &ToolUseBlock) -> Result<ToolResultBlock> {

        let id = tool_use.tool_use_id();
        let name = tool_use.name();
        let input = tool_use.input();
        match name {
            READ_FILE_NAME => {
                let tool_result = read_file(id, input)?;
                Ok(tool_result)
            }
            GENERATE_IMAGE_NAME => {
                let generate_image_result = self.generate_image_from_prompt(input).await;
                if generate_image_result.is_err() {
                    let message = &generate_image_result.err().unwrap().to_string();
                    let tool_result = create_tool_result_block(id, message , ToolResultStatus::Error)?;
                    return Ok(tool_result)
                }

                let tool_result = save_generated_image(id, input, generate_image_result.unwrap())?;
                Ok(tool_result)
            }
            RUN_PYTHON_NAME => {
                let tool_result = run_python(id, input)?;
                Ok(tool_result)
            }
            _ => {
                bail!("The requested tool with name {} does not exist", name)
            }
        }
    }


    // return an array of base64 image string
    async fn generate_image_from_prompt(&mut self, input: &Document) -> Result<Vec<String>> {
        let input_object = input.as_object().context("failed to convert input to object.")?;
        let prompt = input_object.get("prompt")
            .context("prompt is not provided.")?
            .as_string().context("prompt is not string")?;

        let _ = input_object.get("path")
            .context("path is not provided.")?
            .as_string().context("path is not string")?;

        let number_of_images = match input_object.get("numberOfImages").unwrap_or(&Document::Null).as_number() {
            Some(count) => count.to_f32_lossy() as u8,
            None => 1,
        };
        let quality = match input_object.get("quality").unwrap_or(&Document::Null).as_string() {
            Some(q) => serde_json::from_str(q).unwrap_or(IamgeQuality::Standard),
            None => IamgeQuality::Standard,
        };

        let height = match input_object.get("height").unwrap_or(&Document::Null).as_number() {
            Some(height) => height.to_f32_lossy() as u128,
            None => DEFAULT_HEIGHT,
        };

        let width = match input_object.get("width").unwrap_or(&Document::Null).as_number() {
            Some(width) => width.to_f32_lossy() as u128,
            None => DEFAULT_WIDTH,
        };

        let image_config = ImageGenerationConfig {
            number_of_images: Some(number_of_images),
            quality: Some(quality),
            height: Some(height),
            width: Some(width),
        };
        let parameters: ImageGeneratorParameter = ImageGeneratorParameter::new_generate_image_params(
            prompt,
            Some(image_config));
        // println!("{:?}", parameters);

        let parameter_string = serde_json::to_string(&parameters)?;

        let builder = self.bedrock_client
            .invoke_model()
            .model_id(&self.image_model_id)
            .content_type("application/json")
            .body(Blob::new(parameter_string.as_bytes()));

        let response = builder
            .send()
            .await?;
        let body = response.body().clone().into_inner();
        let body_string = str::from_utf8(&body)?;
        let body_value: ImageGeneratorResponse = serde_json::from_str(body_string)?;
        let base64_image_array = body_value.images;

        // println!("Image count: {}", base64_image_array.clone().len());
        Ok(base64_image_array)
    }

}
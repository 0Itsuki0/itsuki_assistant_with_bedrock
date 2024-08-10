pub mod tool;
pub mod bedrock_service;
pub mod terminal_service;
pub mod model_constants;

use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_bedrockruntime::Client;
use bedrock_service::BedrockService;
use clap::{Arg, Command};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::Clear;
use crossterm::{terminal, ExecutableCommand};
use model_constants::{CLAUDE_REGION, REGION_KEY};
use terminal_service::TerminalService;
use core::str;
use std::env;
use std::io::{stdout, Stdout};
use anyhow::Result;


const INTRODUCTION: &str =  "
================================================================================
Welcome to the Itsuki Assistant!
================================================================================
This assistant is powered by AWS Bedrock Claude with the capabilities to
- chat
- generate images
- answer questions on files

Example queries for image generation:
- Generate a cute hello world image in the test folder.
- Generate 2 mathematics image of size 1024 * 1024 in the current folder.

Example queries for questioning regarding files:
- Summarize the content in ./test/test.pdf.

To exit the program, simply type `ESC` or `Ctrl+C`.

*****
Tools are not guranteed to be used for 100% of the time.
Give it another try if the first try does not work out!
*****

P.S.: You have to log in to AWS and have model enabled to use the app! Have fun!
For further details and configuration, please check out GitHub:
https://github.com/0Itsuki0/itsuki_assistant_with_bedrock\r
";

const FINISH: &str =  "
================================================================================
Thank you for checking out!
If you have any feedback or suggestions, please leave me a note at GitHub:
https://github.com/0Itsuki0/itsuki_assistant_with_bedrock
================================================================================
";


#[tokio::main]
async fn main() -> Result<()> {
    let flag_id = "non-stream";
    let command = Command::new("mycmd")
        .arg(
            Arg::new(flag_id)
                .long("non-stream")
                .action(clap::ArgAction::SetTrue)
        );
    let matches = command.get_matches();
    let should_stream = !matches.get_flag(&flag_id);



    let region_string = env::var(REGION_KEY).unwrap_or(CLAUDE_REGION.to_owned());
    let region = Region::new(region_string);

    let region_provider = RegionProviderChain::first_try(region).or_default_provider();
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);
    let mut stdout: Stdout = stdout();

    let mut bedrock_service = BedrockService::new(&client)?;

    let mut terminal_service = TerminalService::new();

    terminal_service.log_info(INTRODUCTION)?;
    terminal::enable_raw_mode()?;
    terminal_service.log_info("You:\r")?;

    let mut user_input: String = String::new();
    let mut empty_input: bool = false;

    'chat: loop {
        let event = event::read()?;

        if let Event::Key(key_event) = event {
            // terminate
            if key_event.code == KeyCode::Esc ||
                (key_event.code == KeyCode::Char('c') && key_event.modifiers == KeyModifiers::CONTROL) {
                break 'chat;
            }

            match key_event.code {
                KeyCode::Char(c) => {
                    if empty_input {
                        empty_input = false;
                        stdout.execute(Clear(terminal::ClearType::CurrentLine))?;
                        terminal_service.log_info("\rYou:\r")?;

                    }
                    terminal_service.log_user_inline(&c)?;
                    user_input.push(c);
                },
                KeyCode::Enter => {
                    if user_input.is_empty() {
                        empty_input = true;
                        terminal_service.clear_line()?;
                        terminal_service.log_info_inline("\rEnter something!\r")?;
                        continue;
                    }

                    terminal_service.log_info_inline("\n\r..... Please wait!\r")?;
                    terminal::disable_raw_mode()?;
                    if should_stream {
                        bedrock_service.run_stream(&user_input).await?;
                    } else {
                        bedrock_service.run(&user_input).await?;
                    }
                    terminal::enable_raw_mode()?;
                    terminal_service.log_info("\rYou:\r")?;
                    user_input = String::from("");
                },
                KeyCode::Backspace | KeyCode::Delete => {
                    terminal_service.delete_char()?;
                    user_input.pop();
                },
                KeyCode::Esc => {
                    break 'chat;
                },
                _ => {},
            }
        }

    };

    terminal::disable_raw_mode()?;
    terminal_service.log_info(FINISH)?;

    Ok(())

}
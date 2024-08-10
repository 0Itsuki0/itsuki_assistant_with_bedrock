# Itsuki Bedrock Assistant

A CLI assistant (chat tool) powered by [AWS Bedrock Converse API](https://docs.aws.amazon.com/bedrock/latest/userguide/conversation-inference.html) with the capabilities to

- chat
- generate images
- answer questions on files


## Install
This tool can be installed using `cargo install bedrock_assistant`.

For further details, please refer to cargo [crate.io](bedrock_assistant).


## Requirements
- AWS Account with Bedrock enabled
    - Default region: `us-east-1`
- Give model access to the models you are planning to use
    - Default Chat Model ID: `anthropic.claude-3-haiku-20240307-v1:0`
    - Default Image Generation Model ID: `amazon.titan-image-generator-v1`
- [Set up AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html)
    - If you use SAML for your AWS account, consider setting up using [`saml2aws`](https://github.com/Versent/saml2aws).

## Optional Set up
- If a different chat model, image generation model, or region other than the default one need to be used, add the following environment variable to your system.
- For Chat Model: `BEDROCK_CHAT_MODEL_ID`
- For Image Generation Model: `BEDROCK_IAMGE_MODEL_ID`
- For Region: `BEDROCK_REGION`


## Usage
- [Sign in through AWS Command Line Interface](https://docs.aws.amazon.com/signin/latest/userguide/command-line-sign-in.html)
- To start the app: run `bedrock_assistant` in the terminal.
    - This app stremas by default. To disable the streaming behavior, pass in `--non-stream` argument.
- To chat: type in your message and press `enter` or `return`.
- To exit the app: press `ESC` or `Ctrl+C`.

### Image generation
Example queries for image generation:
- Generate a cute hello world image in the test folder.
- Generate 2 mathematics image of size 1024 * 1024 in the current folder.

Available configuration for the image to generate:
- Number of images: max of 5. Default to 1.
- Quality: standard or premium. Default to standard.
- height: The height of the image in pixels. Default to 512 pixels.
- width: The width of the image in pixels. Default to 512 pixels.

For more details on the parameters, check out [Bedrock official document](https://docs.aws.amazon.com/bedrock/latest/userguide/model-parameters-titan-image.html#model-parameters-titan-image-api).


### Read File
Example queries for questioning regarding files:
- Summarize the content in ./test/test.pdf.


## Demo

![App Demo](./readme_assets/image_generation_demo.gif)


## Coming Soon
Here are some of the tools/capabilities I am currently working on.

- Image variation
- artifacts for content visualization
- code interpreter


If you have any other suggestiont, leave me a cooment, I would be happy to know!
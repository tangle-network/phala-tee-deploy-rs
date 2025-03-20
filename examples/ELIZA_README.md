# Phala TEE Deploy Examples

This directory contains example applications that demonstrate how to use the Phala TEE deployment library for deploying secure applications in trusted execution environments.

## ELIZA Deployment Examples

### Basic ELIZA Deployment (`eliza_deployment.rs`)

A simple example showing how to deploy an ELIZA chatbot assistant using Phala's TEE infrastructure.

### C3PO ELIZA Deployment (`c3po_eliza_deployment.rs`)

An example showing how to deploy a customized C-3PO themed ELIZA chatbot using Docker Compose:

## Running the Examples

To run any example, use the following command:

```bash
cargo run --example <example_name>
```

## Required Environment Variables

Before running the examples, make sure to set these environment variables:

```bash
# Required for all examples
export PHALA_CLOUD_API_KEY=your_api_key_here

# Optional for ELIZA examples (enables AI capabilities)
export OPENAI_API_KEY=your_openai_key_here

# Optional for C3PO examples
export TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here
export REDPILL_API_KEY=your_redpill_api_key_here
export WALLET_SECRET_SALT=your_wallet_secret_salt_here
```

You can also create a `.env` file in the project root with these variables.

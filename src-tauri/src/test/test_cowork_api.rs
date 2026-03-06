// Rainy Cowork - Legacy API Testing Utility
// Tests legacy rainy-sdk cowork endpoint functionality (v2 compatibility only)

use rainy_sdk::RainyClient;
use std::env;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🔬 Rainy Cowork API Testing");
    println!("================================");

    // Get API key from environment or use test key
    let api_key = env::var("RAINY_API_KEY").unwrap_or_else(|_| {
        println!("❌ No API key found in RAINY_API_KEY");
        println!("Please set the environment variable to test:");
        println!("export RAINY_API_KEY=\"your_api_key_here\"");
        std::process::exit(1);
    });

    println!("🔑 Using API key: {}...", &api_key[..8.min(api_key.len())]);
    println!();

    // Test 1: Create RainyClient
    println!("🧪 Test 1: Creating RainyClient...");
    let client = match RainyClient::with_api_key(&api_key) {
        Ok(client) => {
            println!("✅ RainyClient created successfully");
            client
        }
        Err(e) => {
            println!("❌ Failed to create RainyClient: {}", e);
            return Err(e.into());
        }
    };
    println!();

    // Test 2: Check cowork capabilities
    println!("🧪 Test 2: Getting Cowork Capabilities...");
    match client.get_cowork_capabilities().await {
        Ok(caps) => {
            println!("✅ Cowork Capabilities Retrieved:");
            println!(
                "   Plan: {} ({})",
                caps.profile.plan.name, caps.profile.plan.id
            );
            println!("   Paid: {}", caps.profile.plan.is_paid());
            println!("   Valid: {}", caps.is_valid);
            println!("   Can Make Request: {}", caps.can_make_request());
            println!("   Available Models: {} models", caps.models.len());
            for (i, model) in caps.models.iter().enumerate() {
                println!("     {}. {}", i + 1, model);
            }
            println!("   Features:");
            println!("     Web Research: {}", caps.features.web_research);
            println!("     Document Export: {}", caps.features.document_export);
            println!("     Image Analysis: {}", caps.features.image_analysis);
            println!("     Priority Support: {}", caps.features.priority_support);
            println!(
                "   Usage: {}/{} requests",
                caps.profile.usage.used, caps.profile.usage.limit
            );
            if let Some(msg) = &caps.upgrade_message {
                println!("   Upgrade Message: {}", msg);
            }
        }
        Err(e) => {
            println!("❌ Failed to get cowork capabilities: {}", e);
            return Err(e.into());
        }
    }
    println!();

    // Test 3: Check specific cowork models
    println!("🧪 Test 3: Getting Cowork Models...");
    match client.get_cowork_models().await {
        Ok(models_response) => {
            println!("✅ Cowork Models Retrieved:");
            println!(
                "   Plan: {} ({})",
                models_response.plan, models_response.plan_name
            );
            println!("   Access Level: {}", models_response.model_access_level);
            println!("   Total Models: {}", models_response.total_models);
            for (i, model) in models_response.models.iter().enumerate() {
                println!("     {}. {}", i + 1, model);
            }
        }
        Err(e) => {
            println!("❌ Failed to get cowork models: {}", e);
        }
    }
    println!();

    // Test 4: Test basic chat functionality with different models
    println!("🧪 Test 4: Testing Chat Functionality...");

    let test_models = if client
        .get_cowork_capabilities()
        .await
        .ok()
        .map(|c| c.profile.plan.is_paid())
        .unwrap_or(false)
    {
        // If paid plan, test cowork models
        let caps = client.get_cowork_capabilities().await?;
        caps.models
    } else {
        // If free, test basic models
        vec!["gemini-3-flash-preview".to_string()]
    };

    for model in test_models {
        println!("   🧪 Testing model: {}", model);

        let test_prompt =
            "Hello! Please respond with just 'Model test successful' and nothing else.";

        match client.simple_chat(&model, test_prompt).await {
            Ok(response) => {
                println!("   ✅ {} response: {}", model, response.trim());

                // Note: Image input testing removed as simple_chat_with_image method not available in current SDK
            }
            Err(e) => {
                println!("   ❌ {} failed: {}", model, e);
            }
        }
        println!();
    }

    // Test 5: Check generic available models
    println!("🧪 Test 5: Getting All Available Models...");
    match client.list_available_models().await {
        Ok(models) => {
            println!("✅ Available Models Retrieved:");
            println!("   Total models: {}", models.active_providers.len());
            for (i, model) in models.active_providers.iter().enumerate().take(10) {
                // Limit to first 10
                println!("     {}. {}", i + 1, model);
            }
            if models.active_providers.len() > 10 {
                println!(
                    "     ... and {} more models",
                    models.active_providers.len() - 10
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to list available models: {}", e);
        }
    }

    println!();
    println!("🎯 Testing Complete!");
    println!("================================");

    Ok(())
}

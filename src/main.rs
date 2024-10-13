use chrono::Utc;
use cron::Schedule;
use env_logger::Env;
use reqwest::Client;
use serde::Serialize;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{sleep_until, Duration, Instant};

#[derive(Serialize)]
struct SlackMessage<'a> {
    channel: &'a str,
    text: &'a str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with environment variable support
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Retrieve Slack credentials from environment
    let slack_bot_token = env::var("SLACK_BOT_TOKEN")
        .expect("SLACK_BOT_TOKEN must be set in the environment");
    let slack_channel_id = env::var("SLACK_CHANNEL_ID")
        .expect("SLACK_CHANNEL_ID must be set in the environment");

    // Initialize HTTP client
    let client = Arc::new(Client::new());

    // Define the reminder message
    let reminder_text = "This is your scheduled reminder!";

    // Define cron expressions for every Sunday at 2 PM UTC and 10 PM UTC
    // Option 1: Using numerical representation for Sunday as 7
    let cron_expr_2pm = "0 0 14 * * SUN"; // At 14:00:00 on Sunday
    let cron_expr_10pm = "0 0 22 * * SUN"; // At 22:00:00 on Sunday

    // **Debug Prints (Optional)**
    println!("Parsing cron expression for 2 PM: '{}'", cron_expr_2pm);
    println!("Parsing cron expression for 10 PM: '{}'", cron_expr_10pm);

    // Parse cron expressions with detailed error messages
    let schedule_2pm = Schedule::from_str(cron_expr_2pm)
        .unwrap_or_else(|e| panic!("Invalid cron expression for 2 PM: {}", e));
    let schedule_10pm = Schedule::from_str(cron_expr_10pm)
        .unwrap_or_else(|e| panic!("Invalid cron expression for 10 PM: {}", e));

    // Clone necessary variables for tasks
    let client_clone_2pm = Arc::clone(&client);
    let client_clone_10pm = Arc::clone(&client);
    let token_2pm = slack_bot_token.clone();
    let token_10pm = slack_bot_token.clone();
    let channel_2pm = slack_channel_id.clone();
    let channel_10pm = slack_channel_id.clone();
    let text_2pm = reminder_text.to_string();
    let text_10pm = reminder_text.to_string();

    // Spawn task for 2 PM reminders
    tokio::spawn(async move {
        run_schedule(
            schedule_2pm,
            client_clone_2pm,
            &token_2pm,
            &channel_2pm,
            &text_2pm,
        )
            .await;
    });

    // Spawn task for 10 PM reminders
    tokio::spawn(async move {
        run_schedule(
            schedule_10pm,
            client_clone_10pm,
            &token_10pm,
            &channel_10pm,
            &text_10pm,
        )
            .await;
    });

    println!("Slack Reminder Bot is running...");

    // Keep the program running indefinitely
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn run_schedule(
    schedule: Schedule,
    client: Arc<Client>,
    token: &str,
    channel: &str,
    text: &str,
) {
    let mut upcoming = schedule.upcoming(Utc);
    loop {
        if let Some(datetime) = upcoming.next() {
            let now = Utc::now();
            let duration = datetime - now;
            let duration_std = match duration.to_std() {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Scheduled time is in the past. Skipping.");
                    continue;
                }
            };
            let instant = Instant::now() + duration_std;

            println!("Next reminder scheduled at {}", datetime);

            sleep_until(instant).await;

            // Send the Slack message
            let message = SlackMessage { channel, text };

            match client
                .post("https://slack.com/api/chat.postMessage")
                .bearer_auth(token)
                .json(&message)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("Message sent successfully at {}", Utc::now());
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        eprintln!("Failed to send message: {}", error_text);
                    }
                }
                Err(e) => {
                    eprintln!("Error sending message: {:?}", e);
                }
            }
        } else {
            eprintln!("No upcoming schedule found. Exiting task.");
            break;
        }
    }
}
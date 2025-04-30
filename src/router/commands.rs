use crate::state::{self, Dare, DareDifficulty, RedemptionTask, UserProfile};
use async_trait::async_trait;
use candid::Principal; // Import Principal directly if needed often
use oc_bots_sdk::api::command::{CommandHandler, CommandHandlerRegistry, SuccessResult};
use oc_bots_sdk::api::definition::*;
// Import the specific response type for send_message and the builder trait
use oc_bots_sdk::oc_api::actions::send_message;
use oc_bots_sdk::oc_api::actions::ActionArgsBuilder;
use oc_bots_sdk::oc_api::client::Client;
// Import BotCommandScope instead of CommandScope
use oc_bots_sdk::types::{BotCommandContext, BotCommandScope}; // Keep BotCommandScope
use oc_bots_sdk_canister::env::now;
use oc_bots_sdk_canister::http_command_handler;
use oc_bots_sdk_canister::{CanisterRuntime, HttpRequest, HttpResponse, OPENCHAT_CLIENT_FACTORY};
use rand::seq::SliceRandom;
use std::sync::LazyLock;


// --- Command Handler Structs ---
struct RegisterCmd;
struct DareCmd;
struct SubmitCmd;
struct RedeemCmd;
struct LeaderboardCmd;
struct AddDareCmd;
struct AddTaskCmd;
struct HelpCmd;

// --- Helper to get caller principal from scope ---
fn get_caller_principal(scope: &BotCommandScope) -> Result<Principal, String> {
    match scope {
         // Use correct variant names based on the SDK
        BotCommandScope::PrivateChat { user } => Ok(user.principal),
        BotCommandScope::GroupChat { user, .. } => Ok(user.principal),
        // Add other relevant scope variants if necessary, e.g., Channel
        // BotCommandScope::ChannelChat { user, .. } => Ok(user.principal),
        _ => Err("Command scope does not provide user principal.".to_string()),
    }
}

// --- Command Implementations ---

#[async_trait]
impl CommandHandler<CanisterRuntime> for RegisterCmd {
    fn definition(&self) -> &BotCommandDefinition {
        static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
            name: "register".to_string(),
            description: Some("Register yourself to start playing dares!".to_string()),
            placeholder: None, params: vec![],
            permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
        });
        &DEFINITION
    }

    async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
        let caller = get_caller_principal(&oc_client.context().scope)?;
        match state::get_user(&caller) {
            Some(_) => Err("You are already registered!".to_string()),
            None => {
                let profile = UserProfile {
                    principal: caller, current_dare_id: None, current_streak: 0,
                    longest_streak: 0, dares_completed: 0, last_completion_timestamp: 0,
                };
                state::insert_user(caller, profile);
                 let text = "🎉 Welcome to Darely Bot! You're registered. Use `/dare` to get your first challenge!".to_string();
                 // FIX: Map error from execute_async and extract message on success
                 let response = oc_client
                    .send_text_message(text)
                    .execute_async()
                    .await
                    .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?; // Map the error tuple to String

                match response {
                     // FIX: Use message_id instead of message
                    send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id.into()) }),
                    _ => Err("Failed to send registration confirmation.".to_string()),
                }
            }
        }
    }
}

#[async_trait]
impl CommandHandler<CanisterRuntime> for DareCmd {
     fn definition(&self) -> &BotCommandDefinition {
        static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
            name: "dare".to_string(),
            description: Some("Get a new dare challenge.".to_string()),
            placeholder: Some("Choose difficulty (easy, medium, hard)".to_string()),
            params: vec![ BotCommandParam {
                    name: "difficulty".to_string(),
                    description: Some("Optional: easy, medium, or hard".to_string()),
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 0, max_length: 10,
                        choices: vec![
                            BotCommandOptionChoice { name: "easy".to_string(), value: "easy".to_string() },
                            BotCommandOptionChoice { name: "medium".to_string(), value: "medium".to_string() },
                            BotCommandOptionChoice { name: "hard".to_string(), value: "hard".to_string() },
                        ], multi_line: false,
                    }), required: false, placeholder: Some("e.g., easy".to_string()),
                }],
            permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
        });
        &DEFINITION
    }

     async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
         let caller = get_caller_principal(&oc_client.context().scope)?;
         let mut user_profile = state::get_user(&caller).ok_or("You need to `/register` first!")?;

        if user_profile.current_dare_id.is_some() {
             return Err("You already have an active dare! Use `/submit` when done.".to_string());
        }

        // FIX: Add type annotation
        let difficulty_str: &str = oc_client.context().command.arg("difficulty");
        let requested_difficulty = match difficulty_str.to_lowercase().as_str() {
            "easy" => Some(DareDifficulty::Easy), "medium" => Some(DareDifficulty::Medium),
            "hard" => Some(DareDifficulty::Hard), _ => None,
        };

         let all_dares = state::get_all_dares();
         let available_dares: Vec<_> = all_dares.into_iter()
             .filter(|dare| requested_difficulty.is_none() || Some(dare.difficulty.clone()) == requested_difficulty)
             .collect();

        if available_dares.is_empty() {
             return Err("Sorry, no dares available for that difficulty right now. Admins can use `/add_dare`.".to_string());
        }

        let mut rng = rand::thread_rng();
        let chosen_dare = available_dares.choose(&mut rng).ok_or("Failed to select random dare.")?.clone();

        user_profile.current_dare_id = Some(chosen_dare.id);
        state::insert_user(caller, user_profile);

        let text = format!(
            "🔥 Your new {:?} dare (ID: {}):\n\n{}\n\nUse `/submit <proof>` when completed!",
            chosen_dare.difficulty, chosen_dare.id, chosen_dare.text
        );
         // FIX: Map error from execute_async and extract message on success
         let response = oc_client
            .send_text_message(text)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

        match response {
            send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send dare message.".to_string()),
        }
    }
}

#[async_trait]
impl CommandHandler<CanisterRuntime> for SubmitCmd {
      fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
            name: "submit".to_string(),
            description: Some("Submit proof of completing your current dare.".to_string()),
            placeholder: Some("Provide proof (text, link, etc.)".to_string()),
            params: vec![ BotCommandParam {
                    name: "proof".to_string(),
                    description: Some("Proof of completion (text, image link...). Verification is basic.".to_string()),
                    param_type: BotCommandParamType::StringParam(StringParam {
                         min_length: 1, max_length: 1000, choices: vec![], multi_line: true,
                    }), required: true, placeholder: Some("I did it!".to_string()),
                }],
            permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
        });
        &DEFINITION
    }

     async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
         let caller = get_caller_principal(&oc_client.context().scope)?;
         let _proof = oc_client.context().command.arg("proof");
         let mut user_profile = state::get_user(&caller).ok_or("You need to `/register` first!")?;

        let dare_id = user_profile.current_dare_id.ok_or("No active dare found. Use `/dare`.")?;
        let _dare = state::get_dare(dare_id).ok_or("Internal error: Active dare not found in storage.")?;

        let verification_passed = true;

        if verification_passed {
             user_profile.current_dare_id = None;
             user_profile.current_streak += 1;
             user_profile.dares_completed += 1;
             user_profile.last_completion_timestamp = now();
             if user_profile.current_streak > user_profile.longest_streak {
                 user_profile.longest_streak = user_profile.current_streak;
             }

             let profile_clone = user_profile.clone();
             state::insert_user(caller, user_profile);

            let redemption_threshold = 5;
            let mut response_text = format!(
                "✅ Dare {} submitted! Your current streak is {}.",
                dare_id, profile_clone.current_streak
            );
            let eligible_tasks = state::get_tasks_for_streak(profile_clone.current_streak);
            if !eligible_tasks.is_empty() && profile_clone.current_streak >= redemption_threshold {
                 response_text.push_str(&format!(
                     "\n🏆 Streak goal reached! Use `/redeem` to claim a reward task!"
                 ));
            }
             response_text.push_str("\nUse `/dare` for your next challenge!");

             // FIX: Map error from execute_async and extract message on success
             let response = oc_client
                .send_text_message(response_text)
                .execute_async()
                .await
                .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

            match response {
                 send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
                _ => Err("Failed to send submission confirmation.".to_string()),
            }
        } else {
            Err("Submission could not be verified.".to_string())
        }
    }
}

#[async_trait]
impl CommandHandler<CanisterRuntime> for RedeemCmd {
      fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
            name: "redeem".to_string(),
            description: Some("Redeem your streak for a reward task (if eligible).".to_string()),
            placeholder: None, params: vec![],
            permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
        });
        &DEFINITION
    }

    async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
        let caller = get_caller_principal(&oc_client.context().scope)?;
        let mut user_profile = state::get_user(&caller).ok_or("You need to `/register` first!")?;

        let eligible_tasks = state::get_tasks_for_streak(user_profile.current_streak);

        if eligible_tasks.is_empty() {
            return Err(format!("Sorry, you need a higher streak (current: {}) to redeem a task. Keep going!", user_profile.current_streak));
        }

        let mut rng = rand::thread_rng();
        let chosen_task = eligible_tasks.choose(&mut rng).ok_or("Failed to select redemption task.")?.clone();

        let previous_streak = user_profile.current_streak;
        user_profile.current_streak = 0; // Reset streak

        state::insert_user(caller, user_profile);

        let response_text = format!(
            "🎉 Redeemed! Your streak of {} grants you this task (ID: {}):\n\n{}\n\nYour streak has been reset. Good luck!",
            previous_streak, chosen_task.id, chosen_task.description
        );

         // FIX: Map error from execute_async and extract message on success
         let response = oc_client
            .send_text_message(response_text)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

        match response {
             send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send redemption confirmation.".to_string()),
        }
    }
}


#[async_trait]
impl CommandHandler<CanisterRuntime> for LeaderboardCmd {
      fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
            name: "leaderboard".to_string(),
            description: Some("View the top players by longest streak.".to_string()),
            placeholder: None, params: vec![],
            permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
        });
        &DEFINITION
    }

     async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
        let mut users = state::get_all_users();

        users.sort_by(|(_, a), (_, b)| b.longest_streak.cmp(&a.longest_streak));

        let top_n = 10;
        let mut board = "**🏆 Darely Bot Leaderboard (Longest Streaks) 🏆**\n\n".to_string();

        if users.is_empty() {
            board.push_str("No players yet! Use `/register` to start.");
        } else {
            for (i, (principal, profile)) in users.iter().take(top_n).enumerate() {
                 let principal_str = principal.to_text();
                 let short_principal = if principal_str.len() > 8 {
                     format!("{}...{}", &principal_str[0..5], &principal_str[principal_str.len()-3..])
                } else {
                    principal_str
                };

                 board.push_str(&format!(
                    "{}. {} - Longest: {}, Current: {}\n",
                    i + 1, short_principal, profile.longest_streak, profile.current_streak
                ));
            }
            if users.len() > top_n { board.push_str("\n..."); }
        }

         // FIX: Map error from execute_async and extract message on success
         let response = oc_client
            .send_text_message(board)
            .with_block_level_markdown(true)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

         match response {
            send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send leaderboard.".to_string()),
        }
     }
}


#[async_trait]
impl CommandHandler<CanisterRuntime> for AddDareCmd {
     fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
             name: "add_dare".to_string(), description: Some("ADMIN: Add a new dare.".to_string()),
             placeholder: Some("<difficulty> <dare text>".to_string()),
             params: vec![
                  BotCommandParam {
                      name: "difficulty".to_string(), description: Some("easy, medium, or hard".to_string()),
                      param_type: BotCommandParamType::StringParam(StringParam {
                           min_length: 4, max_length: 6, choices: vec![
                               BotCommandOptionChoice{name:"easy".to_string(), value:"easy".to_string()},
                               BotCommandOptionChoice{name:"medium".to_string(), value:"medium".to_string()},
                               BotCommandOptionChoice{name:"hard".to_string(), value:"hard".to_string()}],
                           multi_line: false, }), required: true, placeholder: Some("medium".to_string()),
                  },
                  BotCommandParam {
                      name: "text".to_string(), description: Some("The text of the dare".to_string()),
                      param_type: BotCommandParamType::StringParam(StringParam {
                           min_length: 5, max_length: 500, choices: vec![], multi_line: true, }),
                      required: true, placeholder: Some("Do 10 jumping jacks".to_string()),
                  }, ],
             permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
         });
         &DEFINITION
     }

      async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
          let caller = get_caller_principal(&oc_client.context().scope)?;
         if !state::is_admin(&caller) { return Err("Only admins can use this command.".to_string()); }

         // FIX: Add type annotation
         let difficulty_str: &str = oc_client.context().command.arg("difficulty");
         let text = oc_client.context().command.arg("text");

          let difficulty = match difficulty_str.to_lowercase().as_str() {
             "easy" => DareDifficulty::Easy, "medium" => DareDifficulty::Medium,
             "hard" => DareDifficulty::Hard, _ => return Err("Invalid difficulty. Use easy, medium, or hard.".to_string()),
         };
          let dare_id = state::get_next_dare_id();
         let new_dare = Dare { id: dare_id, text: text.to_string(), difficulty };
          state::insert_dare(new_dare);
          let response_text = format!("✅ New dare added with ID {}.", dare_id);

           // FIX: Map error from execute_async and extract message on success
           let response = oc_client
            .send_text_message(response_text)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

         match response {
            send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send add_dare confirmation.".to_string()),
        }
     }
}


#[async_trait]
impl CommandHandler<CanisterRuntime> for AddTaskCmd {
     fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
             name: "add_task".to_string(), description: Some("ADMIN: Add a new redemption task.".to_string()),
             placeholder: Some("<required_streak> <task description>".to_string()),
             params: vec![
                  BotCommandParam {
                      name: "required_streak".to_string(), description: Some("Streak needed (enter as number)".to_string()),
                      param_type: BotCommandParamType::StringParam(StringParam {
                           min_length: 1, max_length: 5, choices: vec![], multi_line: false }),
                      required: true, placeholder: Some("5".to_string()),
                  },
                  BotCommandParam {
                      name: "description".to_string(), description: Some("The text/description of the task".to_string()),
                      param_type: BotCommandParamType::StringParam(StringParam {
                           min_length: 5, max_length: 500, choices: vec![], multi_line: true, }),
                      required: true, placeholder: Some("Describe the special reward task".to_string()),
                  }, ],
             permissions: BotPermissions::text_only(), default_role: None, direct_messages: Some(true),
         });
         &DEFINITION
     }

      async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
          let caller = get_caller_principal(&oc_client.context().scope)?;
         if !state::is_admin(&caller) { return Err("Only admins can use this command.".to_string()); }

         // FIX: Add type annotation
         let required_streak_str: &str = oc_client.context().command.arg("required_streak");
         let required_streak: u32 = required_streak_str.parse()
             .map_err(|e| format!("Invalid number for streak: '{}'. Error: {}", required_streak_str, e))?;

         if required_streak < 1 { return Err("Required streak must be 1 or greater.".to_string()); }

         let description = oc_client.context().command.arg("description");

          let task_id = state::get_next_task_id();
          let new_task = RedemptionTask { id: task_id, description: description.to_string(), required_streak };
          state::insert_task(new_task);
          let response_text = format!("✅ New redemption task added with ID {}.", task_id);

           // FIX: Map error from execute_async and extract message on success
           let response = oc_client
            .send_text_message(response_text)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

        match response {
            send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send add_task confirmation.".to_string()),
        }
     }
}


#[async_trait]
impl CommandHandler<CanisterRuntime> for HelpCmd {
     fn definition(&self) -> &BotCommandDefinition {
         static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(|| BotCommandDefinition {
             name: "help".to_string(), description: Some("Show available commands.".to_string()),
             placeholder: None, params: vec![], permissions: BotPermissions::text_only(),
             default_role: None, direct_messages: Some(true),
         });
         &DEFINITION
     }

     async fn execute(&self, oc_client: Client<CanisterRuntime, BotCommandContext>) -> Result<SuccessResult, String> {
          let caller = get_caller_principal(&oc_client.context().scope)?;
          let is_admin = state::is_admin(&caller);

          let mut help_text = "** Darely Bot Commands **\n\n**User Commands:**\n".to_string();
          let mut admin_text = "\n**Admin Commands:**\n".to_string();
          let mut admin_cmds_exist = false;

          for def in COMMANDS.definitions() {
              let line = format!("- `/{}`: {}\n", def.name, def.description.as_deref().unwrap_or(""));
              if def.name.starts_with("add_") || def.name.starts_with("remove_") { // Simple check
                   if is_admin { admin_text.push_str(&line); }
                   admin_cmds_exist = true;
              } else {
                  help_text.push_str(&line);
              }
          }

          if is_admin && admin_cmds_exist { help_text.push_str(&admin_text); }

           // FIX: Map error from execute_async and extract message on success
           let response = oc_client
            .send_text_message(help_text)
            .with_block_level_markdown(true)
            .execute_async()
            .await
            .map_err(|(code, msg)| format!("API Error {}: {}", code, msg))?;

         match response {
            send_message::Response::Success(msg_result) => Ok(SuccessResult { message: Some(msg_result.message_id) }),
            _ => Err("Failed to send help message.".to_string()),
        }
     }
}

// --- Command Registry ---
static COMMANDS: LazyLock<CommandHandlerRegistry<CanisterRuntime>> = LazyLock::new(|| {
    CommandHandlerRegistry::new(OPENCHAT_CLIENT_FACTORY.clone())
        .register(HelpCmd)
        .register(RegisterCmd)
        .register(DareCmd)
        .register(SubmitCmd)
        .register(RedeemCmd)
        .register(LeaderboardCmd)
        .register(AddDareCmd)
        .register(AddTaskCmd)
});

// --- Public Functions ---
pub fn definitions() -> Vec<BotCommandDefinition> { COMMANDS.definitions() }

pub async fn execute(request: HttpRequest) -> HttpResponse {
    let public_key = state::get_oc_public_key();
    let timestamp = now();
    http_command_handler::execute(request, &COMMANDS, &public_key, timestamp).await
}
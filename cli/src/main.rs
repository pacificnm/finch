#![allow(clippy::result_large_err)]

use clap::{Arg, ArgMatches, Command};
use finch_core::{run_command, schwab, CliCommand};
use nest_cli::{AsyncCliCommand, CliApp, CliCommand as CliHostCommand};
use nest_error::NestResult;
use nest_logging::LoggingConfig;

fn main() -> NestResult<()> {
    CliApp::new("finch")
        .with_logging(LoggingConfig::new("finch").with_file("./logs"))
        .command(GreetCommand)
        .command(AboutVersionCommand)
        .command(RecipesCommand)
        .async_command(SchwabCommand)
        .try_run()
}

struct GreetCommand;

impl CliHostCommand for GreetCommand {
    fn name(&self) -> &'static str {
        "greet"
    }

    fn about(&self) -> &'static str {
        "Greet someone"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd.arg(Arg::new("name").default_value("World"))
    }

    fn run(&self, _ctx: &nest_cli::AppContext, matches: &ArgMatches) -> NestResult<()> {
        let name = matches.get_one::<String>("name").unwrap();
        let output = run_command(CliCommand::Greet { name: name.clone() })
            .map_err(nest_error::NestError::unknown)?;
        println!("{output}");
        Ok(())
    }
}

struct AboutVersionCommand;

impl CliHostCommand for AboutVersionCommand {
    fn name(&self) -> &'static str {
        "about-version"
    }

    fn about(&self) -> &'static str {
        "Print the application version"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd
    }

    fn run(&self, _ctx: &nest_cli::AppContext, _matches: &ArgMatches) -> NestResult<()> {
        let output =
            run_command(CliCommand::AboutVersion).map_err(nest_error::NestError::unknown)?;
        println!("{output}");
        Ok(())
    }
}

struct RecipesCommand;

impl CliHostCommand for RecipesCommand {
    fn name(&self) -> &'static str {
        "recipes"
    }

    fn about(&self) -> &'static str {
        "List recipes applied to this app"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd
    }

    fn run(&self, _ctx: &nest_cli::AppContext, _matches: &ArgMatches) -> NestResult<()> {
        let output =
            run_command(CliCommand::ListRecipes).map_err(nest_error::NestError::unknown)?;
        println!("{output}");
        Ok(())
    }
}

/// `finch-cli schwab <...>` — the full Charles Schwab Trader + Market Data
/// API surface, plus OAuth login/logout/status, as one nested command tree.
/// Requires `SCHWAB_APP_KEY`/`SCHWAB_APP_SECRET` in the environment.
struct SchwabCommand;

fn req(name: &'static str) -> Arg {
    Arg::new(name).required(true)
}

fn opt(name: &'static str, long: &'static str) -> Arg {
    Arg::new(name).long(long).required(false)
}

impl SchwabCommand {
    fn configure_auth(cmd: Command) -> Command {
        cmd.subcommand_required(true)
            .subcommand(Command::new("login").about("Run the interactive Schwab OAuth login"))
            .subcommand(Command::new("logout").about("Remove the stored Schwab token"))
            .subcommand(Command::new("status").about("Show whether a Schwab token is stored"))
    }
}

#[async_trait::async_trait]
impl AsyncCliCommand for SchwabCommand {
    fn name(&self) -> &'static str {
        "schwab"
    }

    fn about(&self) -> &'static str {
        "Charles Schwab Trader + Market Data API"
    }

    fn configure(&self, cmd: Command) -> Command {
        cmd.subcommand_required(true)
            .subcommand(Self::configure_auth(
                Command::new("auth").about("Login/logout/status"),
            ))
            .subcommand(Command::new("account-numbers").about("Account-number-to-hash mapping"))
            .subcommand(Command::new("accounts").about("All linked accounts"))
            .subcommand(
                Command::new("account")
                    .about("A single account")
                    .arg(req("account_hash")),
            )
            .subcommand(
                Command::new("orders")
                    .about("Orders for an account")
                    .arg(req("account_hash")),
            )
            .subcommand(
                Command::new("order")
                    .about("A single order")
                    .arg(req("account_hash"))
                    .arg(req("order_id")),
            )
            .subcommand(
                Command::new("place-order")
                    .about("Place a new order (JSON literal or @file.json)")
                    .arg(req("account_hash"))
                    .arg(req("order")),
            )
            .subcommand(
                Command::new("replace-order")
                    .about("Replace an existing order (JSON literal or @file.json)")
                    .arg(req("account_hash"))
                    .arg(req("order_id"))
                    .arg(req("order")),
            )
            .subcommand(
                Command::new("cancel-order")
                    .about("Cancel an order")
                    .arg(req("account_hash"))
                    .arg(req("order_id")),
            )
            .subcommand(
                Command::new("preview-order")
                    .about("Dry-run an order (JSON literal or @file.json)")
                    .arg(req("account_hash"))
                    .arg(req("order")),
            )
            .subcommand(
                Command::new("transactions")
                    .about("Transactions for an account")
                    .arg(req("account_hash")),
            )
            .subcommand(
                Command::new("transaction")
                    .about("A single transaction")
                    .arg(req("account_hash"))
                    .arg(req("transaction_id")),
            )
            .subcommand(Command::new("user-preference").about("Logged-in user's preferences"))
            .subcommand(
                Command::new("quotes")
                    .about("Quotes for one or more symbols")
                    .arg(req("symbols").num_args(1..)),
            )
            .subcommand(
                Command::new("quote")
                    .about("Quote for a single symbol")
                    .arg(req("symbol")),
            )
            .subcommand(
                Command::new("chains")
                    .about("Option chain for a symbol")
                    .arg(req("symbol")),
            )
            .subcommand(
                Command::new("expiration-chain")
                    .about("Option expiration chain for a symbol")
                    .arg(req("symbol")),
            )
            .subcommand(
                Command::new("price-history")
                    .about("Price history for a symbol")
                    .arg(req("symbol"))
                    .arg(opt("period-type", "period-type"))
                    .arg(opt("period", "period"))
                    .arg(opt("frequency-type", "frequency-type"))
                    .arg(opt("frequency", "frequency"))
                    .arg(opt("start-date", "start-date"))
                    .arg(opt("end-date", "end-date")),
            )
            .subcommand(
                Command::new("movers")
                    .about("Top movers for an index")
                    .arg(req("symbol_id"))
                    .arg(opt("sort", "sort"))
                    .arg(opt("frequency", "frequency")),
            )
            .subcommand(
                Command::new("market-hours")
                    .about("Market hours")
                    .arg(opt("markets", "markets"))
                    .arg(opt("date", "date")),
            )
            .subcommand(
                Command::new("instruments")
                    .about("Instrument search")
                    .arg(req("symbol"))
                    .arg(opt("projection", "projection")),
            )
            .subcommand(
                Command::new("instrument")
                    .about("A single instrument by CUSIP")
                    .arg(req("cusip_id")),
            )
    }

    async fn run_async(&self, _ctx: &nest_cli::AppContext, matches: &ArgMatches) -> NestResult<()> {
        let output = run_schwab(matches)
            .await
            .map_err(nest_error::NestError::unknown)?;
        println!("{output}");
        Ok(())
    }
}

async fn run_schwab(matches: &ArgMatches) -> Result<String, String> {
    let (name, sub) = matches
        .subcommand()
        .ok_or_else(|| "missing subcommand".to_string())?;

    let get = |arg: &str| sub.get_one::<String>(arg).unwrap().as_str();
    let get_opt = |arg: &str| sub.get_one::<String>(arg).map(String::as_str);

    match name {
        "auth" => {
            let (auth_name, _) = sub
                .subcommand()
                .ok_or_else(|| "missing `schwab auth` subcommand".to_string())?;
            match auth_name {
                "login" => schwab::auth_login().await,
                "logout" => schwab::auth_logout().await,
                "status" => schwab::auth_status().await,
                other => Err(format!("unknown `schwab auth` subcommand: {other}")),
            }
        }
        "account-numbers" => schwab::account_numbers().await,
        "accounts" => schwab::accounts().await,
        "account" => schwab::account(get("account_hash")).await,
        "orders" => schwab::orders_for_account(get("account_hash")).await,
        "order" => schwab::order(get("account_hash"), get("order_id")).await,
        "place-order" => {
            let order = schwab::parse_order_arg(get("order"))?;
            schwab::place_order(get("account_hash"), order).await
        }
        "replace-order" => {
            let order = schwab::parse_order_arg(get("order"))?;
            schwab::replace_order(get("account_hash"), get("order_id"), order).await
        }
        "cancel-order" => schwab::cancel_order(get("account_hash"), get("order_id")).await,
        "preview-order" => {
            let order = schwab::parse_order_arg(get("order"))?;
            schwab::preview_order(get("account_hash"), order).await
        }
        "transactions" => schwab::transactions(get("account_hash")).await,
        "transaction" => schwab::transaction(get("account_hash"), get("transaction_id")).await,
        "user-preference" => schwab::user_preference().await,
        "quotes" => {
            let symbols: Vec<String> = sub
                .get_many::<String>("symbols")
                .unwrap()
                .cloned()
                .collect();
            schwab::quotes(&symbols).await
        }
        "quote" => schwab::quote(get("symbol")).await,
        "chains" => schwab::option_chain(get("symbol")).await,
        "expiration-chain" => schwab::expiration_chain(get("symbol")).await,
        "price-history" => {
            schwab::price_history(
                get("symbol"),
                get_opt("period-type"),
                get_opt("period"),
                get_opt("frequency-type"),
                get_opt("frequency"),
                get_opt("start-date"),
                get_opt("end-date"),
            )
            .await
        }
        "movers" => schwab::movers(get("symbol_id"), get_opt("sort"), get_opt("frequency")).await,
        "market-hours" => schwab::market_hours(get_opt("markets"), get_opt("date")).await,
        "instruments" => schwab::instruments(get("symbol"), get_opt("projection")).await,
        "instrument" => schwab::instrument_by_cusip(get("cusip_id")).await,
        other => Err(format!("unknown `schwab` subcommand: {other}")),
    }
}

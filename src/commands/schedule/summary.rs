
use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc, Date, NaiveDate, Days};
use chrono_tz::Tz;
use diesel::{sql_types::Time, RunQueryDsl, QueryDsl};
use serenity::{client::Context, model::prelude::command::CommandOptionType};
use std::{sync::Arc, iter::Sum};
use tracing::{instrument, trace, warn};

use diesel::ExpressionMethods;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{AutoComplete, CommandError, SlashCommand},
    config::Config,
    models::{NewReservation, User, ClockStub, DBTransaction},
    ops::user_ops,
    schedule::{Schedule, TimeSlot},
    schema::reservations,
    utils::get_option::{self, get_option}, db::establish_connection,
};
pub(crate) const COMMAND_NAME: &'static str = "summary";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("get employee summaries")
        .create_option(|opt| {
            opt.kind(CommandOptionType::String)
                .name("date")
                .description("Date to get a summary for")
                .set_autocomplete(true)
                .required(true)
        }).create_option(|opt| {
            opt.kind(CommandOptionType::Number)
                .name("page")
                .description("Get the nth page of summaries DEFUALT IS 0")
        })
}


#[derive(Debug)]
pub struct SummaryCommand;
impl SummaryCommand {
    pub fn new() -> Self {
        SummaryCommand {}
    }
}
#[async_trait]
impl SlashCommand for SummaryCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            ..Default::default()
        }
    }
    #[instrument(skip_all, name = "Summary Command", level = "trace")]
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {

        let mut connection = establish_connection();
        let mut options = interaction.data.options.iter();

        let date_str = get_option::<String>(&mut options, "date")?;
        let page_int = get_option::<usize>(&mut options, "page").unwrap_or(0);

        let Ok(mut date) =  NaiveDate::parse_from_str(&date_str, "%Y/%m/%d") else {
            return Err(CommandError::IncorrectParameters("Failed to parse Date".into()))
        };

        let users;
        //Get users
        trace!("Getting users");

        {
            use crate::schema::users::dsl;
            users = dsl::users.load::<User>(&mut connection)?;
        }   
        let mut pay = Vec::new();
        trace!("Calculating Pay for users {:?}",users);
        let beginning_of_day = date.and_hms_opt(0, 0, 0).expect("failed to get beginning of day").and_local_timezone(Utc).unwrap();
        let end_of_day = date.and_hms_opt(23, 59, 59).expect("failed to get end of day").and_local_timezone(Utc).unwrap();
        trace!("Looking for stubs within {} till {}",beginning_of_day,end_of_day);
        for user in users.iter(){
            let stubs;
            {
                use crate::schema::clock_stubs::dsl;
                use diesel::BoolExpressionMethods;
                stubs = dsl::clock_stubs.filter(dsl::user_id.eq(user.id).and(dsl::start_time.between(beginning_of_day,end_of_day))).load::<ClockStub>(&mut connection)?;
            }   
            trace!("Calculating Pay for user {:?} with stubs {:?}",user,stubs);
            let mut total_earned = 0.0;
            let mut total_mins = 0;
            
            for stub in stubs.iter(){
                let Some(end_time) = stub.end_time else{
                    continue;
                };
                let transactions;
                {
                    use crate::schema::transactions::dsl;
                    transactions = dsl::transactions.filter(dsl::clock_stub_id.eq(stub.id)).load::<DBTransaction>(&mut connection)?;
                } 
                trace!("Transactions {:?}",transactions);
                let mut stub_pay = 0.0;
                for transaction in transactions.iter(){
                    let Some(buy_price ) = transaction.buyAvgPrice else{
                        continue;
                    };
                    let Some(sell_price ) = transaction.sellAvgPrice else{
                        continue;
                    };
                    stub_pay += sell_price - buy_price
                    
                }
                let mins = (end_time - stub.start_time).num_minutes();
                trace!("Stub {} Mins {} Pay {}",stub.id,mins,stub_pay);
                total_mins += mins;
                total_earned += stub_pay;

            }
            pay.push((user.tag.clone(),total_mins,total_earned));

        }

        interaction.edit_original_interaction_response(&ctx.http, |i| {
            i.content("Gathered Summary");

            for (tag,mins,earned) in pay.iter().skip((page_int as usize)*25).take(25){
                i.embed(|e|
                    e.title(tag)
                    .field("Mins", mins, true)
                    .field("Earned", earned, true)
                );
            }
            i
        }
        ).await?;


        
        Ok(())
    }
}
#[async_trait]
impl AutoComplete for SummaryCommand {
    #[instrument(skip_all, name = "AutoComplete Summary", level = "trace")]
    async fn auto_complete(
        &self,
        interaction: serenity::model::prelude::interaction::autocomplete::AutocompleteInteraction,
        ctx: Context,
        config: Arc<Config>,
    ) -> Result<(), CommandError> {

        let date = Utc::now().date_naive();
  
        let mut formatted_dates = Vec::new();
        for i in 0..25{
            if let Some(new_date) = date.checked_sub_days(Days::new(i)){
                formatted_dates.push(new_date.format("%Y/%m/%d"));

            }
        }
        
        trace!(
            "Date {}",date
        );


        
        
        interaction
            .create_autocomplete_response(&ctx.http, |a| {
                formatted_dates.iter().take(25).for_each(|str| {
                    a.add_string_choice(str.clone(), str);
                });
                a
            })
            .await?;

        Ok(())
    }
}

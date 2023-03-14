use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Tz;
use diesel::{sql_types::Time, RunQueryDsl, QueryDsl};
use serenity::{client::Context, model::prelude::command::CommandOptionType};
use std::sync::Arc;
use tracing::{instrument, trace};

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
        let users;
        //Get users
        trace!("Getting users");

        {
            use crate::schema::users::dsl;
            users = dsl::users.load::<User>(&mut connection)?;
        }   
        let mut pay = Vec::new();
        trace!("Calculating Pay for users {:?}",users);

        for user in users.iter(){
            let stubs;
            {
                use crate::schema::clock_stubs::dsl;
                stubs = dsl::clock_stubs.filter(dsl::user_id.eq(user.id)).load::<ClockStub>(&mut connection)?;
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
            for (tag,mins,earned) in pay.iter(){
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

use serenity::{
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType,
    },
    prelude::Context,
};

pub async fn send_status(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    msg: &str,
) -> Result<(), serenity::Error> {
    interaction
        .edit_original_interaction_response(&ctx.http, |response| response.content(msg))
        .await?;
    Ok(())
}

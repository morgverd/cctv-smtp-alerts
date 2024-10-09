use mail_parser::Message;

pub(crate) async fn handle_email<'a>(
    message: Message<'a>
) {
    // TODO: Parse the Message to find which event it is, then forward on to webhook.
    unimplemented!();
}
fn main() {
    env_logger::init();

    log::info!("Here we are again");

    sessile_aws_lambda::run(|conn: trillium::Conn| async move {
        conn.ok("hello!").with_header("server", "123")
    });
}

use std::env::set_var;

fn main() {
    set_var("RUST_LOG", "trace");

    env_logger::init();

    log::info!("Here we are again");

    sessile_aws_lambda::run(|conn: trillium::Conn| async move {
        conn.ok("hello!").with_header("server", "123")
    });
}

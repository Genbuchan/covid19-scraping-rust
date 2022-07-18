use crate::structs::app::App;

mod structs;

fn main() {
    let app = App::new();
    app.run();
}

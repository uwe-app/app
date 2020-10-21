use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct List {
    #[structopt(short, long)]
    pub blueprints: bool,
}

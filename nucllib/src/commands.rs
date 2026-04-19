use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Subcommand, Clone, Debug)]
pub enum Commands {
    Enable {
        name: String,
    },
    Disable {
        name: String,
    },
    Start {
        name: String,
    },
    Stop {
        name: String,
    },
    ListUnits,
    Status {
        name: String,
    },
    AddUnit {
        #[arg(long, short)]
        name: String,
        #[arg(
            long, 
            short, 
            trailing_var_arg = true, 
            value_delimiter = ' ',
            num_args = 1..,
            allow_hyphen_values = true,
        )]
        cmd: Vec<String>,
        #[arg(long, short, default_value_t = false)]
        restart: bool,
        #[arg(long, short, default_value_t = false)]
        autostart: bool,
        #[arg(long, short, value_delimiter = ',')]
        dependencies: Option<Vec<String>>,
        #[arg(long, default_value_t = String::from("root"))]
        runas: String
    },
    RemoveUnit {
        name: String,
    },
    Poweroff,
    Reboot,
}

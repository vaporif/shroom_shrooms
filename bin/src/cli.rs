#[derive(clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub seed: Option<u64>,

    /// Dump the Update schedule graph to the given path as DOT and exit.
    /// Convert with `dot -Tsvg <path> -o <path>.svg` to view.
    #[arg(long, value_name = "PATH")]
    pub dump_schedule: Option<std::path::PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_seed_flag() {
        let args = Args::try_parse_from(["kingdom", "--seed", "999"]).unwrap();
        assert_eq!(args.seed, Some(999));
    }

    #[test]
    fn cli_no_seed_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.seed, None);
    }

    #[test]
    fn cli_parses_dump_schedule_flag() {
        let args = Args::try_parse_from(["kingdom", "--dump-schedule", "schedule.dot"]).unwrap();
        assert_eq!(
            args.dump_schedule,
            Some(std::path::PathBuf::from("schedule.dot"))
        );
    }

    #[test]
    fn cli_no_dump_schedule_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.dump_schedule, None);
    }
}

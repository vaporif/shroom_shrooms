#[derive(clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub seed: Option<u64>,

    /// Map width in tiles. Defaults to 220.
    #[arg(long)]
    pub width: Option<i32>,

    /// Map height in tiles. Defaults to 120.
    #[arg(long)]
    pub height: Option<i32>,

    /// Number of rival hives. Defaults to an area-scaled count.
    #[arg(long)]
    pub hives: Option<u32>,

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
    fn cli_parses_width_flag() {
        let args = Args::try_parse_from(["kingdom", "--width", "100"]).unwrap();
        assert_eq!(args.width, Some(100));
    }

    #[test]
    fn cli_no_width_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.width, None);
    }

    #[test]
    fn cli_parses_height_flag() {
        let args = Args::try_parse_from(["kingdom", "--height", "70"]).unwrap();
        assert_eq!(args.height, Some(70));
    }

    #[test]
    fn cli_no_height_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.height, None);
    }

    #[test]
    fn cli_parses_hives_flag() {
        let args = Args::try_parse_from(["kingdom", "--hives", "12"]).unwrap();
        assert_eq!(args.hives, Some(12));
    }

    #[test]
    fn cli_no_hives_flag_yields_none() {
        let args = Args::try_parse_from(["kingdom"]).unwrap();
        assert_eq!(args.hives, None);
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

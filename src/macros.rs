macro_rules! die {
    ($($tt:tt)*) => {
        {
            ::log::error!($($tt)*);
            ::std::process::exit(-1)
        }
    };
}

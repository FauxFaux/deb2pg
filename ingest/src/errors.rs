error_chain! {
    errors {
        InvalidState(msg: String) {
            description("assert!")
            display("invalid state: {}", msg)
        }
    }

    links {
        FaptPkg(::fapt_pkg::Error, ::fapt_pkg::ErrorKind);
        Splayers(::splayers::Error, ::splayers::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Pg(::postgres::error::Error);
        SerdeJson(::serde_json::Error);
    }
}

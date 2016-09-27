// Jianing Yang <jianingy.yang@gmail.com> @ 27 Sep, 2016

error_chain!{

    errors {
        InvalidIpv4Address(t: String) {
            description("invalid ipv4 address")
            display("invalid ipv4 address: {}", t)
        }
        InvalidDatabaseConnectionString(t: String) {
            description("Database connection string is invalid")
            display("Database connection string `{}' is invalid", t)
        }
        DatabaseConnectionError {
            description("cannot connect to database")
            display("cannot connect to database")
        }
        DatabaseError(t: String) {
            description("database error")
            display("database error: {}", t)
        }
        SQLStatementError(t: String) {
            description("SQL statement error")
            display("SQL statement error: {}", t)
        }
    }

}

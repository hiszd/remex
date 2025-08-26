pub mod conn;
pub mod exchange;
use conn::ConnError;
use exchange::ExchangeError;

polyerror::create_error!(pub Error: ConnError, ExchangeError, std::io::Error);

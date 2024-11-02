use mysql::prelude::*;
use mysql::*;
use std::error::Error;

pub fn connect() -> Result<PooledConn, Box<dyn Error>> {
    let adress = "mysql://root:@127.0.0.1:3306/apikeys";
    let pool = Pool::new(adress)?;
    let mut conn = pool.get_conn()?;
    Ok(conn)
}

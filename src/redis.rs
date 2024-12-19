use redis::{ConnectionAddr, ConnectionInfo, RedisConnectionInfo, RedisResult};
use crate::keyspace_info::KeyspaceId;
use crate::KeyspacesInfo;

pub struct RedisConnection {
    connection_info: ConnectionInfo,
    connection: redis::Connection,
}

impl RedisConnection {
    pub fn open(host: String, port: u16, db: KeyspaceId) -> RedisResult<Self> {
        let connection_info = ConnectionInfo {
            addr: ConnectionAddr::Tcp(host, port),
            redis: RedisConnectionInfo {
                db: db.as_i64(),
                ..Default::default()
            },
        };
        Ok(Self {  
            connection_info: connection_info.clone(),
            connection: redis::Client::open(connection_info)?.get_connection()?,
        })
    }

    fn use_connection<F: Fn(&mut redis::Connection) -> RedisResult<T>, T>(&mut self, f: F) -> RedisResult<T> {
        let mut retries = 0;
        loop {
            let result = f(&mut self.connection);
            if result.is_ok() || retries >= 3 {
                return result;
            }
            let err: redis::RedisError = result.err().unwrap();
            eprintln!("Error running command - creating new connection and retrying: {err:?}");
            std::thread::sleep(std::time::Duration::from_secs(match retries {
                0 => 1,
                1 => 2,
                _ => 5,
            }));
            self.connection = redis::Client::open(self.connection_info.clone())?.get_connection()?;
            retries += 1;
        }
    }

    pub fn keyspaces(&mut self) -> RedisResult<KeyspacesInfo> {
        self.use_connection(|conn| redis::cmd("INFO").arg("keyspace").query(conn))
    }

    pub fn scan(&mut self, limit: u64) -> RedisResult<Vec<String>> {
        self.use_connection(|conn| {
            Ok(redis::cmd("SCAN").arg(0).arg("COUNT").arg(limit).clone().iter(conn)?.collect::<Vec<_>>())
        })
    }

    pub fn memory_usage(&mut self, key: &str) -> RedisResult<u64> {
        self.use_connection(|conn| redis::cmd("MEMORY").arg("USAGE").arg(key).arg("SAMPLES").arg(0).query(conn))
    }
}

use redis::{ConnectionAddr, ConnectionInfo, Iter, RedisConnectionInfo, RedisResult};
use crate::keyspace_info::KeyspaceId;
use crate::KeyspacesInfo;

pub struct RedisConnection {
    // pub host: String,
    // pub port: u16,
    // pub db: i64,
    client: redis::Client,
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
            client: redis::Client::open(connection_info)?,
        })
    }

    pub fn keyspaces(&mut self) -> RedisResult<KeyspacesInfo> {
        redis::cmd("INFO").arg("keyspace").query(&mut self.client)
    }

    pub fn scan(&mut self, limit: u64) -> RedisResult<Iter<'_, String>> {
        redis::cmd("SCAN").arg(0).arg("COUNT").arg(limit).clone().iter(&mut self.client)
    }

    pub fn memory_usage(&mut self, key: &str) -> RedisResult<u64> {
        redis::cmd("MEMORY").arg("USAGE").arg(key).arg("SAMPLES").arg(0).query(&mut self.client)
    }
}

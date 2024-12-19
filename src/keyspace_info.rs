use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use redis::{from_redis_value, FromRedisValue, RedisResult, Value};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct KeyspaceId(i64);

impl KeyspaceId {
    pub fn new(id: i64) -> KeyspaceId {
        KeyspaceId(id)
    }

    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl Display for KeyspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct KeyspacesInfo {
    pub keyspaces: HashMap<KeyspaceId, KeyspaceInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyspaceInfo {
    pub keys: u64,
    pub expires: u64,
    pub avg_ttl: u64,
}

impl KeyspaceInfo {
    pub fn from_str(s: &str) -> KeyspaceInfo {
        let mut keys = None;
        let mut expires = None;
        let mut avg_ttl = None;
        for part in s.split(',') {
            let mut kv = part.splitn(2, '=');
            let key = kv.next().unwrap();
            let value = kv.next().unwrap();
            match key {
                "keys" => keys = Some(u64::from_str(value).unwrap()),
                "expires" => expires = Some(u64::from_str(value).unwrap()),
                "avg_ttl" => avg_ttl = Some(u64::from_str(value).unwrap()),
                _ => (),
            }
        }
        KeyspaceInfo {
            keys: keys.unwrap(),
            expires: expires.unwrap(),
            avg_ttl: avg_ttl.unwrap(),
        }
    }
}

impl FromRedisValue for KeyspacesInfo {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let s: String = from_redis_value(v)?;
        let mut keyspaces = HashMap::new();
        for line in s.lines() {
            if line.is_empty() || line == "# Keyspace" {
                continue;
            }
            let mut pair = line.splitn(2, ':');
            let keyspace = pair.next().unwrap();
            let (prefix, number) = keyspace.split_at(2);
            assert_eq!(prefix, "db");
            let number = KeyspaceId::new(i64::from_str(number).unwrap());
            let info = pair.next().unwrap();
            keyspaces.insert(number, KeyspaceInfo::from_str(info));
        }
        Ok(KeyspacesInfo { keyspaces })
    }
}


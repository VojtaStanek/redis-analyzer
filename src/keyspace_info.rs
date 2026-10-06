use redis::{FromRedisValue, ParsingError, Value, from_redis_value};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

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

impl FromStr for KeyspaceInfo {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut keys = None;
        let mut expires = None;
        let mut avg_ttl = None;
        for part in s.split(',') {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("invalid keyspace info part: {part}"))?;
            let parse = |value: &str| {
                u64::from_str(value)
                    .map_err(|e| ParsingError::from(format!("invalid value of {key}: {e}")))
            };
            match key {
                "keys" => keys = Some(parse(value)?),
                "expires" => expires = Some(parse(value)?),
                "avg_ttl" => avg_ttl = Some(parse(value)?),
                _ => (),
            }
        }
        Ok(KeyspaceInfo {
            keys: keys.ok_or("missing keys in keyspace info")?,
            expires: expires.ok_or("missing expires in keyspace info")?,
            avg_ttl: avg_ttl.ok_or("missing avg_ttl in keyspace info")?,
        })
    }
}

impl FromRedisValue for KeyspacesInfo {
    fn from_redis_value(v: Value) -> Result<Self, ParsingError> {
        let s: String = from_redis_value(v)?;
        let mut keyspaces = HashMap::new();
        for line in s.lines() {
            if line.is_empty() || line == "# Keyspace" {
                continue;
            }
            let (keyspace, info) = line
                .split_once(':')
                .ok_or_else(|| format!("invalid keyspace line: {line}"))?;
            let number = keyspace
                .strip_prefix("db")
                .and_then(|n| i64::from_str(n).ok())
                .ok_or_else(|| format!("invalid keyspace name: {keyspace}"))?;
            keyspaces.insert(KeyspaceId::new(number), info.parse()?);
        }
        Ok(KeyspacesInfo { keyspaces })
    }
}

#[cfg(test)]
mod test {
    use super::{KeyspaceId, KeyspacesInfo};
    use redis::{FromRedisValue, Value};

    #[test]
    fn test_parse_keyspaces() {
        let info = "# Keyspace\r\ndb0:keys=10,expires=2,avg_ttl=300\r\ndb3:keys=1,expires=0,avg_ttl=0,subexpiry=0\r\n";
        let parsed = KeyspacesInfo::from_redis_value(Value::BulkString(info.into())).unwrap();
        assert_eq!(parsed.keyspaces.len(), 2);
        let db0 = &parsed.keyspaces[&KeyspaceId::new(0)];
        assert_eq!((db0.keys, db0.expires, db0.avg_ttl), (10, 2, 300));
        assert_eq!(parsed.keyspaces[&KeyspaceId::new(3)].keys, 1);
    }

    #[test]
    fn test_parse_keyspaces_invalid() {
        let info = "# Keyspace\r\nfoo:keys=10\r\n";
        assert!(KeyspacesInfo::from_redis_value(Value::BulkString(info.into())).is_err());
    }
}

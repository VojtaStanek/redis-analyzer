mod keyspace_info;
mod prefix_map;
mod results;
mod redis;
mod results2;

use std::collections::HashMap;
use std::iter::Sum;
use std::ops::Add;
use clap::Parser;
use indicatif::ProgressBar;
use crate::keyspace_info::{KeyspaceId, KeyspacesInfo};
use crate::prefix_map::PrefixMap;
use crate::redis::RedisConnection;
use crate::results::{Datum, Item, Results};

#[derive(Parser, Debug)]
#[clap()]
struct Args {
    /// Redis host
    #[clap(default_value = "127.0.0.1")]
    host: String,
    /// Redis port
    #[clap(default_value = "6379")]
    port: u16,
    /// Output CSV
    #[clap(long)]
    csv: bool,
}

#[derive(Debug, Clone, Copy)]
struct KeyspaceTreeNodeInfo {
    memory_usage: u64,
    count: u64,
}
impl Default for KeyspaceTreeNodeInfo {
    fn default() -> Self {
        KeyspaceTreeNodeInfo {
            memory_usage: 0,
            count: 0,
        }
    }
}
impl Add for KeyspaceTreeNodeInfo {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            memory_usage: self.memory_usage + other.memory_usage,
            count: self.count + other.count,
        }
    }
}

impl Sum for KeyspaceTreeNodeInfo {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Default::default(), |acc, x| acc + x)
    }
}

#[derive(Debug, Clone, Copy)]
struct ExtendedKeyspaceTreeNodeInfo {
    info: KeyspaceTreeNodeInfo,
    estimated_total_count: f64,
    estimated_total_memory_usage: f64,
}
impl Default for ExtendedKeyspaceTreeNodeInfo {
    fn default() -> Self {
        ExtendedKeyspaceTreeNodeInfo {
            info: Default::default(),
            estimated_total_count: 0.0,
            estimated_total_memory_usage: 0.0,
        }
    }
}
impl Add for ExtendedKeyspaceTreeNodeInfo {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            info: self.info + other.info,
            estimated_total_count: self.estimated_total_count + other.estimated_total_count,
            estimated_total_memory_usage: self.estimated_total_memory_usage + other.estimated_total_memory_usage,
        }
    }
}

impl Sum for ExtendedKeyspaceTreeNodeInfo {
    fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = Self>,
    {
        iter.fold(Default::default(), |acc, x| acc + x)
    }
}


fn main() {
    let args = Args::parse();
    let mut connection = RedisConnection::open(args.host.clone(), args.port, KeyspaceId::new(0)).unwrap();
    let keyspaces = match connection.keyspaces() {
        Ok(keyspaces) => keyspaces,
        Err(e) => {
            eprintln!("Redis query failed: {}", e);
            std::process::exit(1);
        }
    };
    eprintln!("Found {} keyspaces", keyspaces.keyspaces.len());

    let sample_sizes = keyspaces.keyspaces.iter().map(|k| (*k.0, match k.1.keys {
        0..=100 => k.1.keys,
        101..=500 => k.1.keys / 5,
        501..=1000 => k.1.keys / 10,
        _ => 200,
    })).collect::<HashMap<_, _>>();

    let samples: HashMap<_, _> = sample_sizes.iter().map(|(&keyspace, &sample_size)| {
        let keyspace_info = keyspaces.keyspaces[&keyspace].clone();
        let total = keyspace_info.keys;
        eprintln!("Getting sample from db{keyspace} - {sample_size} keys of {total} total");
        let mut prefix_map = PrefixMap::default();
        let mut connection = RedisConnection::open(args.host.clone(), args.port, keyspace).unwrap();
        let result = connection.scan(sample_size).unwrap().collect::<Vec<_>>();
        eprintln!("  Scan complete");
        let bar = ProgressBar::new(result.len() as u64);
        for key in result {
            let memory_usage = connection.memory_usage(&key).unwrap();
            prefix_map.insert(key, memory_usage);
            bar.inc(1);
        }
        (keyspace, (sample_size, keyspace_info, prefix_map))
    }).collect();

    let with_info = samples.iter().map(|(keyspace, (sample_size, keyspace_info, prefix_map))| {
        eprintln!("Analyzing db{keyspace}");
        let analyzed_share = *sample_size as f64 / keyspace_info.keys as f64;
        (
            keyspace,
            prefix_map.simplify().transform_to_prefix_map::<ExtendedKeyspaceTreeNodeInfo, _>(&|_key, value, children| {
                let mut out_value = children.iter().map(|(_, map)| map.value).sum::<ExtendedKeyspaceTreeNodeInfo>();
                if value.is_some() {
                    out_value.info.count += 1;
                    out_value.info.memory_usage += value.unwrap();
                }
                (
                    ExtendedKeyspaceTreeNodeInfo {
                        info: out_value.info,
                        estimated_total_memory_usage: out_value.info.memory_usage as f64 / analyzed_share,
                        estimated_total_count: out_value.info.count as f64 / analyzed_share,
                    },
                    children,
                )
            })
        )
    }).collect::<HashMap<_, _>>();


    let merged = PrefixMap::new(
        with_info.iter().map(|(_, it)| it.value).sum::<ExtendedKeyspaceTreeNodeInfo>(),
        with_info.into_iter().map(|(keyspace, map)| (keyspace.to_string(), map)).collect(),
    );

    let results = Results {
        columns: vec![
            "count".to_string(),
            "count_percent".to_string(),
            "memory_usage".to_string(),
            "memory_usage_percent".to_string(),
            "avg_memory_usage".to_string(),
            "estimated_total_count".to_string(),
            "estimated_total_memory_usage".to_string(),
        ],
        items: merged.transform::<(ExtendedKeyspaceTreeNodeInfo, Vec<Item>), _>(&|parent_key, value, children| {
            let mut children = children.into_iter().collect::<Vec<_>>();
            children.sort_by(|(_, (info_l, _)), (_, (info_r, _))| info_l.estimated_total_memory_usage.partial_cmp(&info_r.estimated_total_memory_usage).unwrap());
            let total = children.iter().map(|(_, (count, _))| *count).sum::<ExtendedKeyspaceTreeNodeInfo>();
            (
                value.clone(),
                children
                    .into_iter()
                    .rev()
                    .map(|(key, (info, children))| {
                        Item {
                            name: key[parent_key.len()..].to_string(),
                            columns: {
                                let mut map = HashMap::new();
                                map.insert("count".to_string(), Datum::Count(info.info.count as i64));
                                map.insert("count_percent".to_string(), Datum::Percent(info.info.count as f64 / total.info.count as f64));
                                map.insert("memory_usage".to_string(), Datum::Count(info.info.memory_usage as i64));
                                map.insert("memory_usage_percent".to_string(), Datum::Percent(info.info.memory_usage as f64 / total.info.memory_usage as f64));
                                map.insert("avg_memory_usage".to_string(), Datum::Stat(info.info.memory_usage as f64 / info.info.count as f64));
                                map.insert("estimated_total_count".to_string(), Datum::Stat(info.estimated_total_count));
                                map.insert("estimated_total_memory_usage".to_string(), Datum::Stat(info.estimated_total_memory_usage));
                                map
                            },
                            children: if info.info.count > 2 { children } else { vec![] },
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        }).1,
    };

    if args.csv {
        let mut writer = csv::Writer::from_writer(std::io::stdout());
        results.write_to_csv(&mut writer).unwrap();
    } else {
        println!("{}", results);
    }

}

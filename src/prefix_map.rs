use std::collections::hash_map::Iter;
use std::collections::HashMap;

const KEY_SEPARATORS: [char; 5] = [':', '|', ',', '.', '_'];

type Children<T> = HashMap<String, PrefixMap<T>>;

#[derive(Debug, Clone, Default)]
pub struct PrefixMap<T> {
    pub value: T,
    children: Children<T>,
}

impl <V> PrefixMap<Option<V>> {
    pub fn insert(&mut self, key: String, value: V) {
        let matches = key.match_indices(KEY_SEPARATORS);
        let mut node = self;
        let mut last_prefix = "";
        for (index, _) in matches {
            let prefix = &key[0..(index+1)];
            node = node.children.entry(prefix.to_string()).or_insert(PrefixMap { value: None, children: HashMap::new() });
            last_prefix = prefix;
        }
        if last_prefix == key {
            node.value = Some(value);
        } else {
            node.children.insert(key.to_string(), PrefixMap { value: Some(value), children: HashMap::new() });
        }
    }
}

impl <V: Clone + std::fmt::Debug> PrefixMap<Option<V>> {
    /// Creates a new PrefixMap without nodes with a single child and no value.
    pub fn simplify(&self) -> Self {
        self.replace_nodes::<PrefixMap<Option<V>>, _>(&|prefix, value, children| {
            if value.is_none() && children.len() == 1 {
                let (child_prefix, child) = children.iter().next().unwrap();
                (child_prefix.to_string(), PrefixMap::new(child.value.clone(), child.children.clone()))
            } else {
                (prefix.to_string(), PrefixMap::new(value.clone(), children.clone()))
            }
        })
    }
}

impl <T> PrefixMap<T> {
    pub fn new(value: T, children: Children<T>) -> Self {
        PrefixMap { value, children }
    }

    #[must_use]
    pub fn transform_to_prefix_map<N, F: Fn(&str, &T, Children<N>) -> (N, Children<N>)>(&self, transformer: &F) -> PrefixMap<N> {
        self.transform(&|prefix, value, children| {
            let (new_value, new_children) = transformer(prefix, value, children);
            PrefixMap::new(new_value, new_children)
        })
    }

    #[must_use]
    pub fn transform<R, F: Fn(&str, &T, HashMap<String, R>) -> R>(&self, transformer: &F) -> R {
        self.replace_nodes::<R, _>(&|prefix, value, children| {
            (prefix.to_string(), transformer("", value, children))
        })
    }

    #[must_use]
    pub fn replace_nodes<R, F: Fn(&str, &T, HashMap<String, R>) -> (String, R)>(&self, transformer: &F) -> R {
        self.replace_nodes_inner::<R, F>("", transformer).1
    }

    #[must_use]
    fn replace_nodes_inner<R, F: Fn(&str, &T, HashMap<String, R>) -> (String, R)>(&self, prefix: &str, transformer: &F) -> (String, R) {
        let children: HashMap<String, R> = self.children.iter().map(|(key, child)| {
            child.replace_nodes_inner::<R, _>(key, transformer)
        }).collect();
        transformer(prefix, &self.value, children)
    }

    pub fn iter(&self) -> Iter<'_, String, PrefixMap<T>> {
        self.children.iter()
    }
}

impl <T> IntoIterator for PrefixMap<T> {
    type Item = <HashMap<String, PrefixMap<T>> as IntoIterator>::Item;
    type IntoIter = <HashMap<String, PrefixMap<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}


#[cfg(test)]
mod test {
    #[test]
    fn test1() {
        let mut map = super::PrefixMap::default();
        map.insert("foo:bar".to_string(), ());
        map.insert("foo:bar:".to_string(), ());
        map.insert("foo:bar:1".to_string(), ());
        assert!(map.children.get("foo:").unwrap().children.get("foo:bar").unwrap().value.is_some());
        assert!(map.children.get("foo:").unwrap().children.get("foo:bar:").unwrap().children.get("foo:bar:").is_none());
        assert!(map.children.get("foo:").unwrap().children.get("foo:bar:").unwrap().value.is_some());
        assert!(map.children.get("foo:").unwrap().children.get("foo:bar:").unwrap().children.get("foo:bar:1").unwrap().value.is_some());
    }


    #[test]
    fn test2() {
        let mut map = super::PrefixMap::default();
        map.insert("foo".to_string(), ());
        map.insert("".to_string(), ());
        assert!(map.value.is_some());
        assert!(map.children.get("foo").unwrap().value.is_some());
    }


    #[test]
    fn test_simplify() {
        let simplified = {
            let mut map = super::PrefixMap::default();
            map.insert("foo:bar".to_string(), ());
            map.insert("foo:bar:".to_string(), ());
            map.insert("foo:bar:1".to_string(), ());
            map.insert("foo:bar:2".to_string(), ());
            map
        }.simplify();

        let v1 = simplified.children.get("foo:bar").unwrap();
        assert_eq!(v1.children.len(), 0);
        assert!(v1.value.is_some());

        let v2 = simplified.children.get("foo:bar:").unwrap();
        assert_eq!(v2.children.len(), 2);
        assert!(v2.value.is_some());

        let v3 = v2.children.get("foo:bar:1").unwrap();
        assert_eq!(v3.children.len(), 0);
        assert!(v3.value.is_some());

        let v4 = v2.children.get("foo:bar:2").unwrap();
        assert_eq!(v4.children.len(), 0);
        assert!(v4.value.is_some());
    }


    #[test]
    fn test_transform() {
        let map = {
            let mut map = super::PrefixMap::default();
            map.insert("foo:bar".to_string(), ());
            map.insert("foo:bar:".to_string(), ());
            map.insert("foo:bar:1".to_string(), ());
            map.insert("foo:bar:2".to_string(), ());
            map
        };

        let count = map.transform::<usize, _>(&|_, value, children| {
            value.map_or(0, |_| 1) + children.iter().map(|(_, v)| v).sum::<usize>()
        });

        assert_eq!(count, 4);
    }


    #[test]
    fn test_transform_sum() {
        let map = {
            let mut map = super::PrefixMap::default();
            map.insert("foo:bar".to_string(), 1);
            map.insert("foo:bar:".to_string(), 2);
            map.insert("foo:bar:1".to_string(), 4);
            map.insert("foo:bar:2".to_string(), 8);
            map
        };

        let count = map.transform::<i64, _>(&|_, value, children| {
            value.map_or(0, |v| v) + children.iter().map(|(_, v)| v).sum::<i64>()
        });

        assert_eq!(count, 1 + 2 + 4 + 8);
    }


    #[test]
    fn test_simplify_deep() {
        let simplified = {
            let mut map = super::PrefixMap::default();
            map.insert("bar:1".to_string(), ());
            map.insert("bar:deep:very:deep".to_string(), ());
            map
        }.simplify();

        println!("{:?}", simplified);

        let v1 = simplified.children.get("bar:1").unwrap();
        assert_eq!(v1.children.len(), 0);
        assert!(v1.value.is_some());

        let v2 = simplified.children.get("bar:deep:very:deep").unwrap();
        assert_eq!(v2.children.len(), 0);
        assert!(v2.value.is_some());
    }
}

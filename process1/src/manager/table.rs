use std::rc::Rc;
use std::hash::{Hash};
use std::collections::{HashMap};

pub(super) enum TableOp<'a, K, V> {
    TableInsert(&'a K, &'a V),
    TableRemove(&'a K, &'a V),
}

pub(super) trait TableSubscribe<K, V>:std::fmt::Debug {
    fn filter(&self, op:&TableOp<K, V>) -> bool;
    fn notify(&self, op:&TableOp<K, V>);
}

#[derive(Debug)]
pub(super) struct Table<K, V> {
    data:HashMap<K, V>, // key + value
    subscribers:HashMap<String, Rc<dyn TableSubscribe<K, V>>>, // key: name, value: subscriber
}

impl<K, V> Table<K, V>
where
    K: Eq + Hash + Clone,
{
    pub(super) fn new() -> Table<K, V> {
        Table {
            data:HashMap::new(),
            subscribers:HashMap::new(),
        }
    }

    pub(super) fn insert(&mut self, k:K, v:V) -> Option<V> {
        let key = k.clone();
        let ret = self.data.insert(k, v);
        let value = self.data.get(&key).expect("something inserted is not found.");
        let op = TableOp::TableInsert(&key, value);
        self.notify(&op);
        ret
    }

    pub(super) fn remove(&mut self, k:&K) -> Option<V> {
        let ret = self.data.remove(k);
        if let Some(v) = &ret {
            let op = TableOp::TableRemove(k, v);
            self.notify(&op);
        }
        ret
    }

    pub(super) fn get(&self, k:&K) -> Option<&V> {
        self.data.get(k)
    }

    pub(super) fn subscribe(&mut self, name:String, subscriber:Rc<dyn TableSubscribe<K, V>>) -> Option<Rc<dyn TableSubscribe<K, V>>> {
        self.subscribers.insert(name, subscriber)
    }

    pub(super) fn unsubscribe(&mut self, name:&str) -> Option<Rc<dyn TableSubscribe<K, V>>> {
        self.subscribers.remove(name)
    }

    fn notify(&self, op:&TableOp<'_, K, V>) {
        for (_, subscriber) in self.subscribers.iter() {
            if subscriber.filter(op) {
                subscriber.notify(op);
            }
        }
    }
}

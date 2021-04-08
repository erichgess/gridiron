#![allow(unused)]
use std::collections::HashMap;
use std::cell::RefCell;
use core::hash::Hash;




/**
 * An agent in a group of compute tasks that can generate a result value and
 * messages from its peers. 
 */
trait Automata {

    type Key;

    type Message;

    type Value;

    fn key(&self) -> Self::Key;

    fn messages(&self) -> Vec<(Self::Key, Self::Message)>;

    fn receive(&mut self, message: Self::Message);

    fn is_eligible(&self) -> bool;

    fn value(self) -> Self::Value;
}




/**
 * Execute a group of automata
 */
fn execute<I, C, K, V>(stage: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = C>,
    C: Automata<Key = K, Value = V>,
    K: Hash + Eq,
{
    let mut seen: HashMap<K, C> = HashMap::new();
    let mut undelivered = Vec::new();
    let mut result_values = Vec::new();

    for mut a in stage {

        for (dest, data) in a.messages() {
            if let Some(peer) = seen.get_mut(&dest) {
                peer.receive(data);
                if peer.is_eligible() {
                    result_values.push(seen.remove(&dest).unwrap().value())
                }

            } else {
                undelivered.push((dest, data))
            }
        }

        let key = a.key();
        let mut i = 0;
        while i != undelivered.len() {
            if undelivered[i].0 == key {
                a.receive(undelivered.remove(i).1);
            } else {
                i += 1;
            }
        }

        if a.is_eligible() {
            result_values.push(a.value());
        } else {
            seen.insert(key, a);
        }
    }
    result_values.into_iter()
}

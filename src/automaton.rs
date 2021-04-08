use std::collections::hash_map::{HashMap, Entry};
use core::hash::Hash;




pub enum Receipt<K> {
    Eligible,
    Ineligible(K),
}




/**
 * An agent in a group of compute tasks that can generate a result value and
 * messages from its peers. 
 */
pub trait Automaton {

    type Key;

    type Message;

    type Value;

    fn key(&self) -> Self::Key;

    fn messages(&self) -> Vec<(Self::Key, Self::Message)>;

    fn receive(&mut self, message: (Self::Key, Self::Message)) -> Receipt<Self::Key>;

    fn value(self) -> Self::Value;
}




/**
 * Execute a group of automata
 */
pub fn execute<I, A, K, V>(stage: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Automaton<Key = K, Value = V>,
    K: Hash + Eq,
{
    let mut seen: HashMap<K, A> = HashMap::new();
    let mut undelivered = Vec::new();
    let mut result_values = Vec::new();

    for mut a in stage {

        /*
         * For each of A's messages, either deliver it to the recipient peer, if
         * the peer has already been seen, or otherwise put it in the
         * undelivered box.
         *
         * If any of the recipient peers became eligible upon receiving a
         * message, then execute those peers and collect the result.
         */

        for (dest, data) in a.messages() {
            match seen.entry(dest) {
                Entry::Occupied(entry) => {
                    let (dest, mut peer) = entry.remove_entry();
                    match peer.receive((dest, data)) {
                        Receipt::Eligible => {
                            result_values.push(peer.value());
                        }
                        Receipt::Ineligible(dest) => {
                            seen.insert(dest, peer);
                        }
                    }
                }
                Entry::Vacant(none) => {
                    undelivered.push((none.into_key(), data));
                }
            }
        }

        /*
         * Deliver all of A's messages that have already arrived.
         */

        let dest = a.key();
        let mut i = 0;
        let mut is_eligible = false;
        while i != undelivered.len() {
            if undelivered[i].0 == dest {
                if let Receipt::Eligible = a.receive(undelivered.remove(i)) {
                    is_eligible = true;
                    break;
                }
            } else {
                i += 1;
            }
        }

        /*
         * If A is eligible after receiving its messages, then execute it.
         * Otherwise mark it as seen and process the next automaton.
         */

        if is_eligible {
            result_values.push(a.value());
        } else {
            seen.insert(dest, a);
        }
    }
    result_values.into_iter()
}




/**
 * Execute a group of automata in parallel
 */
pub fn execute_par<'a, I, A: 'a, K, V: 'a>(scope: &rayon::Scope<'a>, stage: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Send + Automaton<Key = K, Value = V>,
    K: Hash + Eq,
    V: Send
{
    use rayon::prelude::*;

    let mut seen: HashMap<K, A> = HashMap::new();
    let mut undelivered = Vec::new();


    let (eligible_sink, eligible_source) = crossbeam_channel::unbounded();
    let (computed_sink, computed_source) = crossbeam_channel::unbounded();


    scope.spawn(move |_| {
        eligible_source
        .into_iter()
        .par_bridge()
        .for_each(|peer: A| {
            computed_sink.send(peer.value()).unwrap();
        });
    });


    for mut a in stage {

        /*
         * For each of A's messages, either deliver it to the recipient peer, if
         * the peer has already been seen, or otherwise put it in the
         * undelivered box.
         *
         * If any of the recipient peers became eligible upon receiving a
         * message, then execute those peers and collect the result.
         */

        for (dest, data) in a.messages() {
            match seen.entry(dest) {
                Entry::Occupied(entry) => {
                    let (dest, mut peer) = entry.remove_entry();
                    match peer.receive((dest, data)) {
                        Receipt::Eligible => {
                            eligible_sink.send(peer).unwrap();
                        }
                        Receipt::Ineligible(dest) => {
                            seen.insert(dest, peer);
                        }
                    }
                }
                Entry::Vacant(none) => {
                    undelivered.push((none.into_key(), data));
                }
            }
        }

        /*
         * Deliver all of A's messages that have already arrived.
         */

        let dest = a.key();
        let mut i = 0;
        let mut is_eligible = false;
        while i != undelivered.len() {
            if undelivered[i].0 == dest {
                if let Receipt::Eligible = a.receive(undelivered.remove(i)) {
                    is_eligible = true;
                    break;
                }
            } else {
                i += 1;
            }
        }

        /*
         * If A is eligible after receiving its messages, then execute it.
         * Otherwise mark it as seen and process the next automaton.
         */

        if is_eligible {
            eligible_sink.send(a).unwrap();
        } else {
            seen.insert(dest, a);
        }
    }
    computed_source.into_iter()
}

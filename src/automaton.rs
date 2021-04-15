use core::hash::Hash;
use std::collections::hash_map::{Entry, HashMap};

/// Returned by [`Automaton::receive`] to indicate whether a task is eligible
/// to be evaluated.
pub enum Status {
    Eligible,
    Ineligible,
}

impl Status {
    pub fn eligible_if(condition: bool) -> Self {
        if condition {
            Self::Eligible
        } else {
            Self::Ineligible
        }
    }
}

/// An agent in a group of compute tasks that can communicate with its peers,
/// and yields a computationally intensive data product. The data product can
/// be another `Automaton` to enable folding of parallel executions. The model
/// uses message passing rather than memory sharing: tasks own their data, and
/// transfer ownership of the message content (and memory buffer) to the
/// recipient. This strategy adheres to the principle of sharing memory by
/// passing messages, rather than passing messages by sharing memory. Memory
/// buffers are _owned_ and _transferred_, never _shared_; buffers don't need
/// to be put under `Arc`, and may be re-used at the discretion of the task on
/// subsequent executions. Heap usage in the `value` method (which is
/// generally run on a worker thread by the executor) can thus be avoided
/// entirely.
///
pub trait Automaton {
    /// The type of the key to uniquely identify this automaton within a
    /// group. Executors will generally require this type to be `Hash + Eq`,
    /// and also `Send` if the executor is multi-threaded.
    type Key;

    /// The type of a message to be passed between the automata. Each stage of
    /// computation requires the receipt of zero or one messages from the
    /// other automata in the group in order to yield a value.
    type Message;

    /// The type of the value yielded by this automaton. Generation of the
    /// yielded value is expected to be CPU-intensive, and may be carried on a
    /// worker thread at the discretion of the executor. For the computation
    /// to proceed requires the initial data on this task, and the messages it
    /// recieved from its peers,
    type Value;

    /// Return the key to uniquely identify this automaton within the group.
    fn key(&self) -> Self::Key;

    /// Return a list of messages to be sent to peers.
    fn messages(&self) -> Vec<(Self::Key, Self::Message)>;

    /// This method must be implemented to receive and store a message from
    /// another task. The receiving task should take ownership of the message
    /// and keep it until a call to `Self::value` is made by the executor.
    /// This method returns a `Status` enum (`Eligible` or `Ineligible`)
    /// indicating if it has now received all of its incoming messages and is
    /// ready to compute a value. This method will be invoked once by the
    /// executor for each incoming message.
    fn receive(&mut self, message: Self::Message) -> Status;

    /// Run the task. CPU-intensive work should be done in this method only.
    /// It is likely to be called on a worker thread, so it should also
    /// minimize creating or dropping memory buffers.
    fn value(self) -> Self::Value;
}

/// Execute a group of tasks in serial.
/// 
pub fn execute<I, A, K, V>(stage: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Automaton<Key = K, Value = V>,
    K: Hash + Eq,
{
    let (eligible_sink, eligible_source) = crossbeam_channel::unbounded();

    coordinate(stage, |a: A| eligible_sink.send(a).unwrap());

    eligible_source.into_iter().map(|peer: A| peer.value())
}

/// Execute a group of tasks in parallel. As tasks are yielded from the input
/// iterator (`flow`), their messages are gathered and delivered to any
/// pending tasks. Those tasks which become eligible upon receiving a message
/// are spawned into the Rayon thread pool. This function returns as soon as
/// the input iterator is exhausted. The output iterator will then yield
/// results until all the tasks have completed in the pool.
/// 
pub fn execute_par<'a, I, A, K, V>(scope: &rayon::ScopeFifo<'a>, flow: I) -> impl Iterator<Item = V>
where
    I: IntoIterator<Item = A>,
    A: Send + Automaton<Key = K, Value = V> + 'a,
    K: Hash + Eq,
    V: Send + 'a,
{
    assert!{
        rayon::current_num_threads() >= 2,
        "automaton::execute_par requires the Rayon pool to be running at least two threads"
    };

    let (sink, source) = crossbeam_channel::unbounded();

    coordinate(flow, |a: A| {
        let sink = sink.clone();
        scope.spawn_fifo(move |_| {
            sink.send(a.value()).unwrap();
        })
    });
    source.into_iter()
}

fn coordinate<I, A, K, V, S>(flow: I, sink: S)
where
    I: IntoIterator<Item = A>,
    A: Automaton<Key = K, Value = V>,
    K: Hash + Eq,
    S: Fn(A),
{
    let mut seen: HashMap<K, A> = HashMap::new();
    let mut undelivered = HashMap::new();

    for mut a in flow {

        // For each of A's messages, either deliver it to the recipient peer,
        // if the peer has already been seen, or otherwise put it in the
        // undelivered box.
        //
        // If any of the recipient peers became eligible upon receiving a
        // message, then send those peers off to be executed.
        // 
        for (dest, data) in a.messages() {
            match seen.entry(dest) {
                Entry::Occupied(mut entry) => {
                    if let Status::Eligible = entry.get_mut().receive(data) {
                        sink(entry.remove())
                    }
                }
                Entry::Vacant(none) => {
                    undelivered
                        .entry(none.into_key())
                        .or_insert(Vec::new())
                        .push(data);
                }
            }
        }

        // Deliver any messages addressed to A that had arrived previously.
        //
        let dest = a.key();
        let mut is_eligible = false;

        if let Some((_, messages)) = undelivered.remove_entry(&dest) {
            for message in messages {
                if let Status::Eligible = a.receive(message) {
                    is_eligible = true;
                    break;
                }
            }
        }

        // If A is eligible after receiving its messages, then send it off to
        // be executed. Otherwise mark it as seen and process the next
        // automaton.
        //
        if is_eligible {
            sink(a);
        } else {
            seen.insert(dest, a);
        }
    }
    assert_eq!(seen.len(), 0);
}

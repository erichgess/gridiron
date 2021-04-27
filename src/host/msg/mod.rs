use serde::{Deserialize, Serialize};

use crate::patch::Patch;

pub enum Signal {
    Stop,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    id: usize,
    d: Patch,
}

impl Request {
    pub fn new(id: usize, d: &Patch) -> Request {
        Request { id, d: d.clone() }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn data(&self) -> &Patch {
        &self.d
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    status: Status,
}

impl Response {
    pub fn new(s: Status) -> Response {
        Response { status: s }
    }

    pub fn status(&self) -> Status {
        self.status
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Status {
    Good(usize),
    Bad,
}

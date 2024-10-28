use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QMemory {
    pub mem: Vec<Vec<u8>>,
    // the qubits shoud be the same with the Qubits
    pub qubits: usize,
    pub capacity: usize,
    pub current_pos: usize,
}

impl Default for QMemory {
    fn default() -> Self {
        QMemory {
            mem: vec![vec![0; 20]; 20],
            qubits: 20,
            capacity: 20,
            current_pos: 0,
        }
    }
}

impl QMemory {
    pub fn new(qubits: usize, capacity: usize) -> Self {
        QMemory {
            mem: vec![vec![0; qubits]; capacity],
            qubits,
            capacity,
            current_pos: 0,
        }
    }

    pub fn read_file(path: &str) -> Self {
        let reader: Box<dyn Read> = Box::new(std::fs::File::open(path).unwrap());
        serde_pickle::from_reader(reader, Default::default()).unwrap()
    }

    pub fn dump_file(&self, path: &str) {
        let mut writer: Box<dyn Write> = Box::new(std::fs::File::create(path).unwrap());

        serde_pickle::to_writer(
            &mut writer,
            self,
            serde_pickle::SerOptions::new().proto_v2(),
        )
        .unwrap();
    }

    /// update the capacity of the result, will make the result to be empty
    pub fn update_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.current_pos = 0;
        self.mem = vec![vec![0; self.qubits]; capacity];
    }

    /// update the qubits of the result, will make the result to be empty
    pub fn update_qubits(&mut self, qubits: usize) {
        self.qubits = qubits;
        self.current_pos = 0;
        self.mem = vec![vec![0; qubits]; self.capacity];
    }

    pub fn update_results(&mut self, string: &String) {
        let mut mz_res: Vec<u8> = vec![0; self.qubits];
        for (i, c) in string.chars().rev().enumerate() {
            if c == '1' {
                mz_res[self.qubits - i - 1] = 1;
            } else {
                mz_res[self.qubits - i - 1] = 0;
            }
        }
        self.mem[self.current_pos] = mz_res;
        self.current_pos += 1;
        self.current_pos %= self.capacity;
    }
}

#[derive(Debug, Clone)]
pub struct QResgister {
    pub qubits: Vec<bool>,
    pub idle: usize,
}

impl Default for QResgister {
    fn default() -> Self {
        QResgister {
            qubits: vec![false; 20],
            idle: 20,
        }
    }
}

impl QResgister {
    pub fn new(num_qubits: usize) -> Self {
        QResgister {
            qubits: vec![false; num_qubits],
            idle: num_qubits,
        }
    }

    pub fn update_qubits(&mut self, num_qubits: usize) {
        self.qubits = vec![false; num_qubits];
        self.idle = num_qubits;
    }

    pub fn update_idle(&mut self, idle: usize) {
        self.idle = idle;
    }
}

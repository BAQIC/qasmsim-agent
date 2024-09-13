use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeasureResult {
    pub results: Vec<Vec<u8>>,
    pub qbits: usize,
    pub capacity: usize,
    pub current_pos: usize,
}

impl Default for MeasureResult {
    fn default() -> Self {
        MeasureResult {
            results: vec![vec![0; 0]; 0],
            qbits: 0,
            capacity: 0,
            current_pos: 0,
        }
    }
}

impl MeasureResult {
    pub fn new(qbits: usize, capacity: usize) -> Self {
        MeasureResult {
            results: vec![vec![0; qbits]; capacity],
            qbits,
            capacity,
            current_pos: 0,
        }
    }

    pub fn read_file(path: &str) -> Self {
        let result = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(result.as_str()).unwrap()
    }

    pub fn dump_file(&self, path: &str) {
        let result = serde_json::to_string(self).unwrap();
        std::fs::write(path, result).unwrap();
    }

    /// update the capacity of the result, will make the result to be empty
    pub fn update_capaicity(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.current_pos = 0;
        self.results = vec![vec![0; self.qbits]; capacity];
    }

    /// update the qbits of the result, will make the result to be empty
    pub fn update_qbits(&mut self, qbits: usize) {
        self.qbits = qbits;
        self.current_pos = 0;
        self.results = vec![vec![0; qbits]; self.capacity];
    }
}

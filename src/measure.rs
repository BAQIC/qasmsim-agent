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
            results: vec![vec![0; 20]; 20],
            qbits: 20,
            capacity: 20,
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

    pub fn update_results(&mut self, str: &String) {
        let mut mz_res: Vec<u8> = vec![0; self.qbits];
        for (i, c) in str.chars().rev().enumerate() {
            if c == '1' {
                mz_res[self.qbits - i - 1] = 1;
            } else {
                mz_res[self.qbits - i - 1] = 0;
            }
        }

        self.results[self.current_pos] = mz_res;
        self.current_pos += 1;
        self.current_pos = self.current_pos % self.capacity;
    }
}

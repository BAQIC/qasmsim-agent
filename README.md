# qasmsim-agent

This is a simple agent that can be used to simulate a quantum computer using the [qasmsim](https://github.com/delapuente/qasmsim).

## API format

The agent expects a `application/x-www-form-urlencoded` POST request with the following parameters:

- `qasm`: The QASM code to be executed.
- `shots`: The number of shots to be executed.
- `mode`: The mode of the simulation.
- `vars`: The variables to be used in the simulation.

The response will be a JSON object with the following fields:

- `Memory`: A list of the results of the measurements.

## Example

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "task_id": "test",
  "shots": 3,
  "qasm": "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[8];creg c[8];\nx q[0];\ny q[1];\nh q[2];\nmeasure q[0] -> c[0];\nmeasure q[1] -> c[1];\nmeasure q[2] -> c[2];",
  "mode": "sequence"
}' http://127.0.0.1:3003/submit

{"Result":[["00000111","00000011","00000111"]],"init_position":9}
```

Update classical storage info:
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "qbits": 30,
  "capacity": 30
}' http://127.0.0.1:3003/update

{"Result":"Update classical info with ClassicalInfo { qbits: Some(30), capacity: Some(30) }"}
```

Query measure result:
```bash
curl 'http://127.0.0.1:3003/get_measure?pos=1'

{"Results":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1]}
```

## Run with docker

pull docker image from github:
```bash
docker pull ghcr.io/baqic/qasmsim-agent:main
```

run container with some parameters:
```bash
docker run -d -p 3003:3003 --env QUAFU_IP=127.0.0.1:3003 --env MEASURE_PATH="./measure.json" ghcr.io/baqic/qasmsim-agent:main
```
NOTE: environment variables `QUAFU_IP` and `MEASURE_PATH` are required. `MEASURE_PATH` is for classical storage.

check classical storage:
```bash
# for previous, measure path is /measure.json
docker exec -it container_id /bin/cat MEASURE_PATH
{"results":[[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]],"qbits":20,"capacity":20,"current_pos":12}
```

use a persistent volume:
```bash
docker run -d -p 3003:3003 --env QUAFU_IP=127.0.0.1:3003 --env MEASURE_PATH="/storage/measure.json" -v agent-volume:/storage ghcr.io/baqic/qasmsim-agent:main
```
Next time, you can use the same volume to keep the classical storage.

## Example VQE
  
```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "task_id": "test",
  "shots": 10,
  "qasm": "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[8];creg c[8];\nx q[0];\ny q[1];\nh q[2];\nmeasure q[0] -> c[0];\nmeasure q[1] -> c[1];\nry(variable_01) q[3];\nmeasure q[2] -> c[2];",
  "mode": "vqe",
  "vars": "{\"variable_01\": [0.0, 20.0],\n\"variable_02\": [0.0, 30.0]}",
  "iterations": 10
}' http://127.0.0.1:3003/submit
```

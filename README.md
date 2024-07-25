# qasmsim-agent

This is a simple agent that can be used to simulate a quantum computer using the [qasmsim](https://github.com/delapuente/qasmsim).

## API format

The agent expects a `application/x-www-form-urlencoded` POST request with the following parameters:

- `qasm`: The QASM code to be executed.
- `shots`: The number of shots to be executed.

The response will be a JSON object with the following fields:

- `Memory`: A list of the results of the measurements.

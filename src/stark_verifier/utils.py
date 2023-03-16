import json
import subprocess

PWD = subprocess.run(['pwd'], capture_output=True).stdout[:-1].decode("utf-8")
PROOF_PATH = f'{PWD}/proofs/fib.bin'
PARSER_PATH = f'{PWD}/bin/stark_parser'
CAIRO_PRIME = 2**251 + 17 * 2**192 + 1


def write_into_memory(ptr, json_data, segments):
    addr = ptr
    if hasattr(ptr, 'address_'):
        addr = ptr.address_

    my_array = json.loads(json_data)
    # Note the following:
    # - Addresses are stored as `Relocatable` values in the Cairo VM.
    # - The "+" operator is overloaded to perform pointer arithmetics.
    # - Felts are hex encoded starting with "0x". The virtual addresses are encoded as decimals.
    my_memory = [(int(x, 16) if x.startswith('0x') else addr + int(x))
                 for x in my_array]
    segments.write_arg(addr, my_memory)
    # print(addr, my_memory)


def index_of(elements_ptr, n_elements, element, memory):
    for i in range(n_elements):
        if (memory[elements_ptr + i] == element):
            return i
    return CAIRO_PRIME-1


def read_fri_queries_proofs(positions_ptr, fri_queries_proof_ptr, num_queries, memory, segments):
    positions = to_json_array(positions_ptr, num_queries, memory)

    completed_process = subprocess.run(
        [PARSER_PATH, PROOF_PATH, 'fri-queries', positions],
        capture_output=True)

    json_data = completed_process.stdout
    write_into_memory(fri_queries_proof_ptr, json_data, segments)


def to_json_array(arr_ptr, arr_length, memory):
    arr = []
    for i in range(arr_length):
        arr.append(memory[arr_ptr + i])
    return json.dumps(arr)

from starkware.cairo.common.alloc import alloc

from stark_verifier.utils import Vec

struct MemEntry {
    address: felt,
    value: felt,
}

struct ProgramOutputs {
    stack_len: felt,
    stack: felt*,
    overflow_addrs_len: felt,
    overflow_addrs: felt*,
}

struct PublicInputs {
    program_hash_len: felt,
    program_hash: felt*,
    stack_inputs_len: felt,
    stack_inputs: felt*,
    outputs: ProgramOutputs,
}

func read_public_inputs() -> PublicInputs* {
    let (pub_inputs_ptr: PublicInputs*) = alloc();
    %{
        from src.stark_verifier.utils import write_into_memory
        write_into_memory(ids.pub_inputs_ptr, json_data, segments)
    %}
    return pub_inputs_ptr;
}

func read_mem_values(mem: MemEntry*, address: felt, length: felt, output: felt*) {
    if (length == 0) {
        return ();
    }
    assert mem.address = address;
    assert output[0] = mem.value;
    return read_mem_values(mem=&mem[1], address=address + 1, length=length - 1, output=&output[1]);
}

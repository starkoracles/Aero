struct BaseElementGpu {
    val: u64
}

@group(0) @binding(0) var<storage, read> to_multiply: array<vec2<BaseElementGpu>>;
@group(0) @bindings(1) var<storage, read_write> results: array<BaseElementGpu>;

@compute
@workgroup_size(64)
fn mul_g(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let total = arrayLength(&to_multiply);
    var i : u32 = 0u;
    loop {
        if (i >= total) {
            break;
        }
        let r = to_multiply[i].x * to_multiply[i].y;
        results[i].val = r;
        continuing {
            i = i + 1u;
        }
    }
}
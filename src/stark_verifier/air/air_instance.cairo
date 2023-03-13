from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.pow import pow

from stark_verifier.crypto.random import (
    PublicCoin,
    draw_elements,
    draw_pair,
    random_coin_new,
    hash_elements,
    reseed,
    seed_with_pub_inputs,
)
from stark_verifier.air.pub_inputs import PublicInputs
from stark_verifier.air.stark_proof import ProofContext, ProofOptions, StarkProof

struct AirInstance {
    // Layout
    main_segment_width: felt,
    aux_trace_width: felt,
    aux_segment_widths: felt*,
    aux_segment_rands: felt*,
    num_aux_segments: felt,
    // Context
    options: ProofOptions,
    context: ProofContext,
    num_transition_constraints: felt,
    num_assertions: felt,
    ce_blowup_factor: felt,
    // eval_frame_size: felt,
    trace_domain_generator: felt,
    lde_domain_generator: felt,
    // Public input
    pub_inputs: PublicInputs*,
}

// Coefficients used in construction of constraint composition polynomial
struct ConstraintCompositionCoefficients {
    // Transition constraints (alpha and beta)
    transition_a: felt*,
    transition_b: felt*,
    // Boundary constraints (alpha and beta)
    boundary_a: felt*,
    boundary_b: felt*,
}

struct TraceCoefficients {
    n_values: felt,
    values: felt*,
}

// Coefficients used in construction of DEEP composition polynomial
struct DeepCompositionCoefficients {
    // Trace polynomial composition coefficients $\alpha_i$, $\beta_i$, and $\gamma_i$
    trace: TraceCoefficients*,
    // Constraint column polynomial composition coefficients $\delta_j$
    constraints: felt*,
    // Degree adjustment composition coefficients $\lambda$ and $\mu$
    degree: (felt, felt),
}

func air_instance_new{
    range_check_ptr
}(
    proof: StarkProof*,
    pub_inputs: PublicInputs*,
    options: ProofOptions
) -> AirInstance {
    alloc_locals;
    let (aux_segment_widths: felt*) = alloc();
    let (aux_segment_rands: felt*) = alloc();

    let trace_domain_generator = [range_check_ptr];
    let lde_domain_generator = [range_check_ptr + 1];
    let range_check_ptr = range_check_ptr + 2;

    %{
        # TODO this is insecure
        # 2-adic root of unity for field with modulus $2^{64} - 2^{32} + 1$
        TWO_ADICITY = 32
        # 2^32 root of unity
        G = 1753635133440165772

        PG = 18446744069414584321 # 2^64 - 2^32 - 1
        power = pow(2, TWO_ADICITY - ids.proof.context.log_trace_length, PG)
        ids.trace_domain_generator = pow(G, power, PG)

        log_lde_domain_size = ids.options.log_blowup_factor + ids.proof.context.log_trace_length
        power = pow(2, TWO_ADICITY - log_lde_domain_size, PG)
        ids.lde_domain_generator = pow(G, power, PG)
        print("blow_up", ids.proof.context.options.blowup_factor)
    %}

    // TODO: Make configurable for other VMs and custom AIRs
    let res = AirInstance(
        main_segment_width=72,
        aux_trace_width=9,
        aux_segment_widths=aux_segment_widths,
        aux_segment_rands=aux_segment_rands,
        num_aux_segments=1,
        options=options,
        context=proof.context,
        num_transition_constraints=49,
        num_assertions=7,
        ce_blowup_factor=proof.context.options.blowup_factor,
        // eval_frame_size=2,
        trace_domain_generator=trace_domain_generator,
        lde_domain_generator=lde_domain_generator,
        pub_inputs=pub_inputs,
    );
    return res;
}

// Returns coefficients needed to construct the constraint composition polynomial
func get_constraint_composition_coefficients{
    range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*, public_coin: PublicCoin
}(air: AirInstance) -> ConstraintCompositionCoefficients {
    alloc_locals;

    let (t_coefficients_a: felt*) = alloc();
    let (t_coefficients_b: felt*) = alloc();
    let num_constraints = air.num_transition_constraints;
    draw_pairs(
        n_pairs=num_constraints, coefficients_a=t_coefficients_a, coefficients_b=t_coefficients_b
    );

    let (b_coefficients_a: felt*) = alloc();
    let (b_coefficients_b: felt*) = alloc();
    let num_assertions = air.num_assertions;
    draw_pairs(
        n_pairs=num_assertions, coefficients_a=b_coefficients_a, coefficients_b=b_coefficients_b
    );

    let res = ConstraintCompositionCoefficients(
        transition_a=t_coefficients_a,
        transition_b=t_coefficients_b,
        boundary_a=b_coefficients_a,
        boundary_b=b_coefficients_b,
    );

    return res;
}

// Returns coefficients needed to construct the DEEP composition polynomial
func get_deep_composition_coefficients{
    range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*, public_coin: PublicCoin
}(air: AirInstance) -> DeepCompositionCoefficients {
    alloc_locals;

    let (t_coefficients: TraceCoefficients*) = alloc();
    set_trace_coefficients(
        n_vec= air.context.trace_layout.main_segment_width + air.aux_trace_width,
        n_coefficients=3, // TODO: Why is 3 hardcoded?
        coefficients=t_coefficients,
    );

    let (c_coefficients: felt*) = alloc();
    draw_elements(n_elements=air.ce_blowup_factor, elements=c_coefficients); 

    let (lambda, mu) = draw_pair();

    let res = DeepCompositionCoefficients(
        trace=t_coefficients, constraints=c_coefficients, degree=(lambda, mu)
    );
    return res;
}

func set_trace_coefficients{
    range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*, public_coin: PublicCoin
}(n_vec: felt, n_coefficients: felt, coefficients: TraceCoefficients*) {
    if (n_vec == 0) {
        return ();
    }
    // Create a new TraceCoefficients object
    let (values) = alloc();
    assert coefficients[0] = TraceCoefficients(n_coefficients, values);
    
    // Fill it with random elements
    draw_elements(n_elements=n_coefficients, elements=values);

    // Recurse
    set_trace_coefficients(
        n_vec=n_vec - 1, n_coefficients=n_coefficients, coefficients=&coefficients[1]
    );
    return ();
}

// Returns the next pair of pseudo-random field elements, and adds them to the
// list of coefficients
func draw_pairs{
    range_check_ptr, blake2s_ptr: felt*, bitwise_ptr: BitwiseBuiltin*, public_coin: PublicCoin
}(n_pairs: felt, coefficients_a: felt*, coefficients_b: felt*) {
    
    if (n_pairs == 0) {
        return ();
    }
    
    let (num1, num2) = draw_pair();
    assert coefficients_a[0] = num1;
    assert coefficients_b[0] = num2;

    return draw_pairs(
        n_pairs=n_pairs - 1, coefficients_a=coefficients_a + 1, coefficients_b=coefficients_b + 1
    );
}

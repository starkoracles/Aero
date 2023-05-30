use crate::utils::{
    from_uint8array, set_once_logger, to_uint8array, ConstraintComputeResult,
    ConstraintComputeWorkItem, FeltWrapper,
};
use js_sys::Uint8Array;
use log::debug;
use miden_air::ProcessorAir;
use miden_core::{Felt, FieldElement};
use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};
use winter_air::Air;
use winter_prover::{ConstraintEvaluationTable, ConstraintEvaluator, StarkDomain};

pub fn constraint_compute(work_item: &ConstraintComputeWorkItem) -> Result<Uint8Array, JsValue> {
    let air = ProcessorAir::new(
        work_item.trace_info.clone(),
        work_item.public_inputs.clone(),
        work_item.proof_options.clone(),
    );
    // 2 ----- evaluate constraints -----------------------------------------------------------
    // evaluate constraints specified by the AIR over the constraint evaluation domain, and
    // compute random linear combinations of these evaluations using coefficients drawn from
    // the channel; this step evaluates only constraint numerators, thus, only constraints with
    // identical denominators are merged together. the results are saved into a constraint
    // evaluation table where each column contains merged evaluations of constraints with
    // identical denominators.
    let constraint_coeffs = work_item.constraint_coeffs.clone();
    let aux_trace_rand_elements = work_item.aux_rand_elements.clone();
    // build a list of constraint divisors; currently, all transition constraints have the same
    // divisor which we put at the front of the list; boundary constraint divisors are appended
    // after that
    let evaluator: ConstraintEvaluator<_, Felt> =
        ConstraintEvaluator::new(&air, aux_trace_rand_elements, constraint_coeffs);
    // build a list of constraint divisors; currently, all transition constraints have the same
    // divisor which we put at the front of the list; boundary constraint divisors are appended
    // after that
    let mut divisors = vec![evaluator.transition_constraints.divisor().clone()];
    divisors.append(&mut evaluator.boundary_constraints.get_divisors());

    let domain = StarkDomain::new(&air);
    let trace_table = &work_item.trace_lde;

    // allocate space for constraint evaluations; when we are in debug mode, we also allocate
    // memory to hold all transition constraint evaluations (before they are merged into a
    // single value) so that we can check their degrees later
    #[cfg(not(debug_assertions))]
    let mut evaluation_table = ConstraintEvaluationTable::<Felt>::new(&domain, divisors);
    #[cfg(debug_assertions)]
    let mut evaluation_table = ConstraintEvaluationTable::<Felt>::new(
        &domain,
        divisors,
        &evaluator.transition_constraints,
    );
    let frag_num = work_item.computation_fragment.num_fragments;
    let mut fragments = evaluation_table.fragments(frag_num);
    let frag = &mut fragments[work_item.computation_fragment.fragment_offset];
    evaluator.evaluate_fragment(&trace_table, &domain, frag);
    debug!(
        "done processing constraints for batch {}",
        work_item.computation_fragment.fragment_offset
    );

    let mut evaluations = vec![vec![FeltWrapper(Felt::ZERO); frag.num_rows()]; frag.num_columns()];
    for i in 0..frag.num_columns() {
        for j in 0..frag.num_rows() {
            evaluations[i][j] = FeltWrapper(frag.evaluations[i][j]);
        }
    }

    let response = ConstraintComputeResult {
        frag_index: frag.offset(),
        frag_num,
        constraint_evaluations: evaluations,
    };

    Ok(to_uint8array(&response))
}

#[wasm_bindgen]
pub fn constraint_entry_point(msg: MessageEvent) -> Result<(), JsValue> {
    set_once_logger();
    if let Ok(work_item) =
        from_uint8array::<ConstraintComputeWorkItem>(&Uint8Array::new(&msg.data()))
    {
        debug!(
            "Constraint worker received work item: {:?}",
            work_item.computation_fragment.fragment_offset
        );
        let response = constraint_compute(&work_item)?;
        let global_scope = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
        global_scope.post_message(&response)?;
    } else {
        debug!("Constraint worker received invalid work item");
    }
    Ok(())
}

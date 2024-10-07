#![feature(allocator_api)]
#![feature(generic_const_exprs)]

use boojum::cs::gates::ConstantAllocatableCS;
use boojum::cs::traits::cs::ConstraintSystem;

use boojum::cs::gates::testing_tools::test_evaluator;
use boojum::dag::CircuitResolverOpts;
use boojum::field::Field;
use std::alloc::Global;

use boojum::cs::gates::poseidon2::*;
use boojum::worker::Worker;
type F = boojum::field::goldilocks::GoldilocksField;
type RCfg = <boojum::config::DevCSConfig as CSConfig>::ResolverConfig;
use boojum::implementations::poseidon2::Poseidon2Goldilocks;

type Poseidon2Gate = Poseidon2FlattenedGate<F, 8, 12, 4, Poseidon2Goldilocks>;
use boojum::cs::cs_builder_reference::*;

use boojum::config::{CSConfig, DevCSConfig};
use boojum::cs::cs_builder::new_builder;
use boojum::cs::gates::constant_allocator::ConstantsAllocatorGate;
use boojum::cs::gates::nop_gate::NopGate;
use boojum::cs::gates::poseidon2::Poseidon2FlattenedGate;
use boojum::cs::traits::gate::GatePlacementStrategy;
use boojum::cs::{CSGeometry, Place, Variable};

fn main() {
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 80,
        num_witness_columns: 80,
        num_constant_columns: 10,
        max_allowed_constraint_degree: 8,
    };

    let builder_impl = CsReferenceImplementationBuilder::<F, F, DevCSConfig>::new(geometry, 8);
    let builder = new_builder::<_, F>(builder_impl);

    let builder =
        Poseidon2Gate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);
    let builder = ConstantsAllocatorGate::configure_builder(
        builder,
        GatePlacementStrategy::UseGeneralPurposeColumns,
    );
    let builder =
        NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);

    let mut owned_cs = builder.build(CircuitResolverOpts::new(128));

    let cs = &mut owned_cs;

    let mut inputs = [Variable::placeholder(); 8];
    let mut state = [F::ZERO; 12];
    for (idx, dst) in inputs.iter_mut().enumerate() {
        let value = F::from_u64_with_reduction(idx as u64);
        let var = cs.alloc_single_variable_from_witness(value);
        state[idx] = value;
        *dst = var;
    }

    let capacity_var = cs.allocate_constant(F::ZERO);

    let outputs = [capacity_var; 4];

    let round_function_result = Poseidon2Gate::compute_round_function(cs, inputs, outputs);

    use boojum::implementations::poseidon2::poseidon2_permutation;
    poseidon2_permutation(&mut state);

    println!("Out of circuit result = {:?}", state);

    let circuit_result = cs
        .get_value_for_multiple(Place::from_variables(round_function_result))
        .wait()
        .unwrap();

    println!("Circuit result = {:?}", circuit_result);

    assert_eq!(circuit_result, state);

    drop(cs);
    owned_cs.pad_and_shrink();

    let worker = Worker::new();

    println!("Checking if satisfied");
    let mut owned_cs = owned_cs.into_assembly::<Global>();
    assert!(owned_cs.check_if_satisfied(&worker));
}

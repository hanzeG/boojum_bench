#![feature(allocator_api)]
#![feature(generic_const_exprs)]
use chrono::Utc;
use serde_json::json;
use std::alloc::Global;
use std::fs::{create_dir_all, File};
use std::io::Write;

use boojum::cs::gates::*;
use boojum::cs::implementations::reference_cs::CSReferenceImplementation;
use boojum::field::traits::field::Field;
use boojum::worker::Worker;
type F = boojum::field::goldilocks::GoldilocksField;

use boojum::cs::gates::poseidon::PoseidonFlattenedGate;
use boojum::implementations::poseidon_goldilocks_naive::PoseidonGoldilocks;
type PoseidonGate = PoseidonFlattenedGate<F, 8, 12, 4, PoseidonGoldilocks>;
use boojum::config::{CSConfig, DevCSConfig};
use boojum::cs::cs_builder::new_builder;
use boojum::cs::cs_builder_reference::*;
use boojum::cs::traits::cs::ConstraintSystem;
use boojum::cs::traits::gate::GatePlacementStrategy;
use boojum::cs::{CSGeometry, Place, Variable};
use boojum::dag::CircuitResolverOpts;

/// Gets the current timestamp in a human-readable format.
fn get_timestamp() -> String {
    let now = Utc::now();
    now.format("%Y-%m-%d_%H-%M-%S").to_string()
}

fn main() {
    let geometry = CSGeometry {
        num_columns_under_copy_permutation: 80,
        num_witness_columns: 0,
        num_constant_columns: 8,
        max_allowed_constraint_degree: 8,
    };

    let builder_impl = CsReferenceImplementationBuilder::<F, F, DevCSConfig>::new(geometry, 8);

    let builder = new_builder::<_, F>(builder_impl);

    let builder =
        PoseidonGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);
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

    let round_function_result = PoseidonGate::compute_round_function(cs, inputs, outputs);

    use boojum::implementations::poseidon_goldilocks_naive::*;
    poseidon_permutation(&mut state);

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

    // // Configure proof generation settings
    // let lde_factor_to_use = 32;
    // let mut proof_config = ProofConfig::default();
    // proof_config.fri_lde_factor = lde_factor_to_use;
    // proof_config.pow_bits = 0;

    // // Generate the proof and verification key
    // let (proof, vk) = owned_cs.prove_one_shot::<
    //         GoldilocksExt2,
    //         GoldilocksPoisedonTranscript,
    //         GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
    //         NoPow,
    //     >(&worker, proof_config, ());

    // // Create the data directory if it does not exist
    // create_dir_all("data").expect("Unable to create data directory");

    // // Generate a timestamp for file naming
    // let timestamp = get_timestamp();

    // // Serialize proof to JSON and write to data/proof_TIMESTAMP.json
    // let proof_json = json!(proof);
    // let mut proof_file = File::create(format!("data/proof_{}.json", timestamp))
    //     .expect("Unable to create data/proof.json");
    // writeln!(proof_file, "{}", proof_json).expect("Unable to write to data/proof.json");

    // // Serialize verification key to JSON and write to data/vk_TIMESTAMP.json
    // let vk_json = json!(vk);
    // let mut vk_file =
    //     File::create(format!("data/vk_{}.json", timestamp)).expect("Unable to create data/vk.json");
    // writeln!(vk_file, "{}", vk_json).expect("Unable to write to data/vk.json");

    // // Serialize public inputs to JSON and write to data/public_TIMESTAMP.json
    // let public_inputs_json = json!(proof.public_inputs);
    // let mut pubs_file = File::create(format!("data/public_{}.json", timestamp))
    //     .expect("Unable to create data/public.json");
    // writeln!(pubs_file, "{}", public_inputs_json).expect("Unable to write to data/public.json");

    // println!(
    //     "Proof, verification key, and public inputs have been generated in the data directory."
    // );

    // // Verification process
    // let builder_impl = CsVerifierBuilder::<F, GoldilocksExt2>::new_from_parameters(geometry);
    // let builder = new_builder::<_, F>(builder_impl);
    // let builder =
    //     Poseidon2Gate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);
    // let builder = ConstantsAllocatorGate::configure_builder(
    //     builder,
    //     GatePlacementStrategy::UseGeneralPurposeColumns,
    // );
    // let builder =
    //     NopGate::configure_builder(builder, GatePlacementStrategy::UseGeneralPurposeColumns);
    // let verifier = builder.build(());

    // // Verify the proof
    // let is_valid = verifier.verify::<
    //     GoldilocksPoseidonSponge<AbsorptionModeOverwrite>,
    //     GoldilocksPoisedonTranscript,
    //     NoPow
    // >(
    //     (),
    //     &vk,
    //     &proof,
    // );

    // println!("Is the proof valid? {}", is_valid);
}

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

use ark_crypto_primitives::{
    crh::{
        poseidon::constraints::{CRHGadget, CRHParametersVar},
        poseidon::CRH,
        CRHScheme, CRHSchemeGadget,
    },
    sponge::{poseidon::PoseidonConfig, Absorb},
};
use ark_ff::PrimeField;
use ark_pallas::{constraints::GVar, Fr, Projective};
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::{alloc::AllocVar, fields::FieldVar};
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};
use ark_vesta::{constraints::GVar as GVar2, Projective as Projective2};
use core::marker::PhantomData;
use std::time::Instant;

use folding_schemes::commitment::pedersen::Pedersen;
use folding_schemes::folding::nova::Nova;
use folding_schemes::frontend::FCircuit;
use folding_schemes::{Error, FoldingScheme};
mod utils;
use folding_schemes::transcript::poseidon::poseidon_test_config;
use utils::test_nova_setup;

/// This is the circuit that we want to fold, it implements the FCircuit trait. The parameter z_i
/// denotes the current state which contains 2 elements, and z_{i+1} denotes the next state which
/// we get by applying the step.
/// In this example we set the state to be the previous state together with an external input, and
/// the new state is an array which contains the new state and a zero which will be ignored.
///
///        w_1     w_2     w_3     w_4     
///        │       │       │       │      
///        ▼       ▼       ▼       ▼      
///       ┌─┐     ┌─┐     ┌─┐     ┌─┐     
/// ─────►│F├────►│F├────►│F├────►│F├────►
///  z_1  └─┘ z_2 └─┘ z_3 └─┘ z_4 └─┘ z_5
///
///
/// where each F is:
///    w_i                                        
///     │     ┌────────────────────┐              
///     │     │FCircuit            │              
///     │     │                    │              
///     └────►│ h =Hash(z_i[0],w_i)│              
///           │ │ =Hash(v, w_i)    │              
///  ────────►│ │                  ├───────►      
/// z_i=[v,0] │ └──►z_{i+1}=[h, 0] │ z_{i+1}=[h,0]
///           │                    │              
///           └────────────────────┘
///
#[derive(Clone, Debug)]
pub struct ExternalInputsCircuits<F: PrimeField>
where
    F: Absorb,
{
    _f: PhantomData<F>,
    poseidon_config: PoseidonConfig<F>,
}
impl<F: PrimeField> FCircuit<F> for ExternalInputsCircuits<F>
where
    F: Absorb,
{
    type Params = PoseidonConfig<F>;

    fn new(params: Self::Params) -> Self {
        Self {
            _f: PhantomData,
            poseidon_config: params,
        }
    }
    fn state_len(&self) -> usize {
        2
    }

    /// computes the next state values in place, assigning z_{i+1} into z_i, and computing the new
    /// z_{i+1}
    fn step_native(&self, z_i: Vec<F>) -> Result<Vec<F>, Error> {
        let input = [z_i[0], z_i[1]];
        let out = CRH::<F>::evaluate(&self.poseidon_config, input).unwrap();
        Ok(vec![out, F::zero()])
    }

    /// generates the constraints for the step of F for the given z_i
    fn generate_step_constraints(
        &self,
        cs: ConstraintSystemRef<F>,
        z_i: Vec<FpVar<F>>,
    ) -> Result<Vec<FpVar<F>>, SynthesisError> {
        let crh_params =
            CRHParametersVar::<F>::new_constant(cs.clone(), self.poseidon_config.clone())?;

        let input = [z_i[0].clone(), z_i[1].clone()];
        let out = CRHGadget::<F>::evaluate(&crh_params, &input)?;
        Ok(vec![out, FpVar::<F>::zero()])
    }
}

/// cargo test --example external_inputs
#[cfg(test)]
pub mod tests {
    use super::*;
    use ark_r1cs_std::R1CSVar;
    use ark_relations::r1cs::ConstraintSystem;

    // test to check that the ExternalInputsCircuits computes the same values inside and outside the circuit
    #[test]
    fn test_f_circuit() {
        let poseidon_config = poseidon_test_config::<Fr>();

        let cs = ConstraintSystem::<Fr>::new_ref();

        let circuit = ExternalInputsCircuits::<Fr>::new(poseidon_config);
        let z_i = vec![Fr::from(1_u32), Fr::from(2_u32)];

        let z_i1 = circuit.step_native(z_i.clone()).unwrap();

        let z_iVar = Vec::<FpVar<Fr>>::new_witness(cs.clone(), || Ok(z_i)).unwrap();
        let computed_z_i1Var = circuit
            .generate_step_constraints(cs.clone(), z_iVar.clone())
            .unwrap();
        assert_eq!(computed_z_i1Var.value().unwrap(), z_i1);
    }
}

/// cargo run --release --example external_inputs
fn main() {
    let num_steps = 10;
    let initial_state = vec![Fr::from(1_u32), Fr::from(2_u32)];

    let poseidon_config = poseidon_test_config::<Fr>();
    let F_circuit = ExternalInputsCircuits::<Fr>::new(poseidon_config);

    println!("Prepare Nova ProverParams & VerifierParams");
    let (prover_params, verifier_params) =
        test_nova_setup::<ExternalInputsCircuits<Fr>>(F_circuit.clone());

    /// The idea here is that eventually we could replace the next line chunk that defines the
    /// `type NOVA = Nova<...>` by using another folding scheme that fulfills the `FoldingScheme`
    /// trait, and the rest of our code would be working without needing to be updated.
    type NOVA = Nova<
        Projective,
        GVar,
        Projective2,
        GVar2,
        ExternalInputsCircuits<Fr>,
        Pedersen<Projective>,
        Pedersen<Projective2>,
    >;

    println!("Initialize FoldingScheme");
    let mut folding_scheme = NOVA::init(&prover_params, F_circuit, initial_state.clone()).unwrap();

    // compute a step of the IVC
    for i in 0..num_steps {
        let start = Instant::now();
        folding_scheme.prove_step().unwrap();
        println!("Nova::prove_step {}: {:?}", i, start.elapsed());
    }
    println!(
        "state at last step (after {} iterations): {:?}",
        num_steps,
        folding_scheme.state()
    );

    let (running_instance, incoming_instance, cyclefold_instance) = folding_scheme.instances();

    println!("Run the Nova's IVC verifier");
    NOVA::verify(
        verifier_params,
        initial_state.clone(),
        folding_scheme.state(), // latest state
        Fr::from(num_steps as u32),
        running_instance,
        incoming_instance,
        cyclefold_instance,
    )
    .unwrap();
}

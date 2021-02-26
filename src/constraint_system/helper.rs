// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{PlookupComposer, StandardComposer};
use crate::commitment_scheme::kzg10::PublicParameters;
use crate::plookup::{PlookupTable4Arity, PreprocessedTable4Arity};
use crate::proof_system::{PlookupProver, PlookupVerifier, Prover, Verifier};
use anyhow::{Error, Result};
use dusk_bls12_381::BlsScalar;

/// Adds dummy constraints using arithmetic gates
pub(crate) fn dummy_gadget(n: usize, composer: &mut StandardComposer) {
    let one = BlsScalar::one();

    let var_one = composer.add_input(one);

    for _ in 0..n {
        composer.big_add(
            var_one.into(),
            var_one.into(),
            None,
            BlsScalar::zero(),
            BlsScalar::zero(),
        );
    }
}

/// Adds dummy constraints using arithmetic gates
pub(crate) fn dummy_gadget_plookup(n: usize, composer: &mut PlookupComposer) {
    let one = BlsScalar::one();

    let var_one = composer.add_input(one);

    for _ in 0..n {
        composer.big_add(
            var_one.into(),
            var_one.into(),
            None,
            BlsScalar::zero(),
            BlsScalar::zero(),
        );
    }
}

/// Takes a generic gadget function with no auxillary input and
/// tests whether it passes an end-to-end test
pub(crate) fn gadget_tester(
    gadget: fn(composer: &mut StandardComposer),
    n: usize,
) -> Result<(), Error> {
    // Common View
    let public_parameters = PublicParameters::setup(2 * n, &mut rand::thread_rng())?;
    // Provers View
    let (proof, public_inputs) = {
        // Create a prover struct
        let mut prover = Prover::new(b"demo");

        // Additionally key the transcript
        prover.key_transcript(b"key", b"additional seed information");

        // Add gadgets
        gadget(&mut prover.mut_cs());

        // Commit Key
        let (ck, _) = public_parameters.trim(2 * prover.cs.circuit_size().next_power_of_two())?;

        // Preprocess circuit
        prover.preprocess(&ck)?;

        // Once the prove method is called, the public inputs are cleared
        // So pre-fetch these before calling Prove
        let public_inputs = prover.cs.public_inputs.clone();

        // Compute Proof
        (prover.prove(&ck)?, public_inputs)
    };
    // Verifiers view
    //
    // Create a Verifier object
    let mut verifier = Verifier::new(b"demo");

    // Additionally key the transcript
    verifier.key_transcript(b"key", b"additional seed information");

    // Add gadgets
    gadget(&mut verifier.mut_cs());

    // Compute Commit and Verifier Key
    let (ck, vk) = public_parameters.trim(verifier.cs.circuit_size().next_power_of_two())?;

    // Preprocess circuit
    verifier.preprocess(&ck)?;

    // Verify proof
    verifier.verify(&proof, &vk, &public_inputs)
}

/// Takes a generic gadget function and
/// tests whether it passes an end-to-end test
pub(crate) fn gadget_plookup_tester(
    gadget: fn(composer: &mut PlookupComposer),
    n: usize,
    lookup_table: PlookupTable4Arity,
) -> Result<(), Error> {
    // Common View
    let public_parameters = PublicParameters::setup(2 * n, &mut rand::thread_rng()).unwrap();
    // Provers View
    let (proof, public_inputs) = {
        // Create a prover struct
        let mut prover = PlookupProver::new(b"demo");
        // Additionally key the transcript
        prover.key_transcript(b"key", b"additional seed information");

        // Add gadgets
        gadget(&mut prover.mut_cs());

        // Commit Key
        let (ck, _) = public_parameters
            .trim(2 * prover.cs.circuit_size().next_power_of_two())
            .unwrap();

        // This ought to be added to shared preprocessing
        if prover.mut_cs().w_l.len() < lookup_table.0.len() {
            for i in 0..(lookup_table.0.len() - prover.mut_cs().w_l.len()) {
                prover.mut_cs().add_one_dummy_constraint();
            }
        }

        // Preprocess circuit
        prover.preprocess(&ck).unwrap();

        // Once the prove method is called, the public inputs are cleared
        // So pre-fetch these before calling Prove
        let public_inputs = prover.cs.public_inputs.clone();

        // Compute Proof
        (
            prover.prove_with_table(&ck, &lookup_table).unwrap(),
            public_inputs,
        )
    };
    // Verifiers view
    //
    // Create a Verifier object
    let mut verifier = PlookupVerifier::new(b"demo");

    // Additionally key the transcript
    verifier.key_transcript(b"key", b"additional seed information");

    // Add gadgets
    gadget(&mut verifier.mut_cs());

    // This ought to be added to shared preprocessing
    if verifier.mut_cs().w_l.len() < lookup_table.0.len() {
        for i in 0..(lookup_table.0.len() - verifier.mut_cs().w_l.len()) {
            verifier.mut_cs().add_one_dummy_constraint();
        }
    }

    // Compute Commit and Verifier Key
    let (ck, vk) = public_parameters
        .trim(verifier.cs.circuit_size().next_power_of_two())
        .unwrap();

    // Preprocess circuit
    verifier.preprocess(&ck).unwrap();

    // Verify proof
    verifier.verify(&proof, &vk, &public_inputs, &lookup_table)
}

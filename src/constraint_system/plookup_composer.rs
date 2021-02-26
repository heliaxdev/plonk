// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The `Composer` is a Trait that is actually defining some kind of
//! Circuit Builder for PLONK.
//!
//! In that sense, here we have the implementation of the `PlookupComposer`
//! which has been designed in order to provide the maximum amount of performance
//! while having a big scope in utility terms.
//!
//! It allows us not only to build Add and Mul constraints but also to build
//! ECC op. gates, Range checks, Logical gates (Bitwise ops) etc.

// Gate fn's have a large number of attributes but
// it is intended to be like this in order to provide
// maximum performance and minimum circuit sizes.
#![allow(clippy::too_many_arguments)]

use super::cs_errors::PreProcessingError;
use crate::constraint_system::Variable;
use crate::permutation::Permutation;
use dusk_bls12_381::BlsScalar;
use std::collections::HashMap;

/// A composer is a circuit builder
/// and will dictate how a circuit is built
/// We will have a default Composer called `PlookupComposer`
#[derive(Debug)]
pub struct PlookupComposer {
    // n represents the number of arithmetic gates in the circuit
    pub(crate) n: usize,

    // Selector vectors
    //
    // Multiplier selector
    pub(crate) q_m: Vec<BlsScalar>,
    // Left wire selector
    pub(crate) q_l: Vec<BlsScalar>,
    // Right wire selector
    pub(crate) q_r: Vec<BlsScalar>,
    // Output wire selector
    pub(crate) q_o: Vec<BlsScalar>,
    // Fourth wire selector
    pub(crate) q_4: Vec<BlsScalar>,
    // Constant wire selector
    pub(crate) q_c: Vec<BlsScalar>,
    // Arithmetic wire selector
    pub(crate) q_arith: Vec<BlsScalar>,
    // Range selector
    pub(crate) q_range: Vec<BlsScalar>,
    // Logic selector
    pub(crate) q_logic: Vec<BlsScalar>,
    // Fixed base group addition selector
    pub(crate) q_fixed_group_add: Vec<BlsScalar>,
    // Variable base group addition selector
    pub(crate) q_variable_group_add: Vec<BlsScalar>,
    // Plookup gate wire selector
    pub(crate) q_lookup: Vec<BlsScalar>,
    /// Public inputs vector
    pub public_inputs: Vec<BlsScalar>,

    // Witness vectors
    pub(crate) w_l: Vec<Variable>,
    pub(crate) w_r: Vec<Variable>,
    pub(crate) w_o: Vec<Variable>,
    pub(crate) w_4: Vec<Variable>,

    /// A zero variable that is a part of the circuit description.
    /// We reserve a variable to be zero in the system
    /// This is so that when a gate only uses three wires, we set the fourth wire to be
    /// the variable that references zero
    pub(crate) zero_var: Variable,

    // These are the actual variable values
    // N.B. They should not be exposed to the end user once added into the composer
    pub(crate) variables: HashMap<Variable, BlsScalar>,

    pub(crate) perm: Permutation,
}

impl PlookupComposer {
    /// Returns the number of gates in the circuit
    pub fn circuit_size(&self) -> usize {
        self.n
    }
}

impl Default for PlookupComposer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlookupComposer {
    /// Generates a new empty `PlookupComposer` with all of it's fields
    /// set to hold an initial capacity of 0.
    ///
    /// # Warning
    ///
    /// The usage of this may cause lots of re-allocations since the `Composer`
    /// holds `Vec` for every polynomial, and these will need to be re-allocated
    /// each time the circuit grows considerably.
    pub fn new() -> Self {
        PlookupComposer::with_expected_size(0)
    }

    /// Fixes a variable in the witness to be a part of the circuit description.
    pub fn add_witness_to_circuit_description(&mut self, value: BlsScalar) -> Variable {
        let var = self.add_input(value);
        self.constrain_to_constant(var, value, BlsScalar::zero());
        var
    }

    /// Creates a new circuit with an expected circuit size.
    /// This will allow for less reallocations when building the circuit
    /// since the `Vec`s will already have an appropriate allocation at the
    /// beginning of the composing stage.
    pub fn with_expected_size(expected_size: usize) -> Self {
        let mut composer = PlookupComposer {
            n: 0,

            q_m: Vec::with_capacity(expected_size),
            q_l: Vec::with_capacity(expected_size),
            q_r: Vec::with_capacity(expected_size),
            q_o: Vec::with_capacity(expected_size),
            q_c: Vec::with_capacity(expected_size),
            q_4: Vec::with_capacity(expected_size),
            q_arith: Vec::with_capacity(expected_size),
            q_range: Vec::with_capacity(expected_size),
            q_logic: Vec::with_capacity(expected_size),
            q_fixed_group_add: Vec::with_capacity(expected_size),
            q_variable_group_add: Vec::with_capacity(expected_size),
            q_lookup: Vec::with_capacity(expected_size),
            public_inputs: Vec::with_capacity(expected_size),

            w_l: Vec::with_capacity(expected_size),
            w_r: Vec::with_capacity(expected_size),
            w_o: Vec::with_capacity(expected_size),
            w_4: Vec::with_capacity(expected_size),

            zero_var: Variable(0),

            variables: HashMap::with_capacity(expected_size),

            perm: Permutation::new(),
        };

        // Reserve the first variable to be zero
        composer.zero_var = composer.add_witness_to_circuit_description(BlsScalar::zero());

        // Add dummy constraints
        composer.add_dummy_constraints();

        composer
    }

    /// Add Input first calls the `Permutation` struct
    /// to generate and allocate a new variable `var`.
    /// The composer then links the Variable to the BlsScalar
    /// and returns the Variable for use in the system.
    pub fn add_input(&mut self, s: BlsScalar) -> Variable {
        // Get a new Variable from the permutation
        let var = self.perm.new_variable();
        // The composer now links the BlsScalar to the Variable returned from the Permutation
        self.variables.insert(var, s);

        var
    }

    /// This pushes the result of a lookup read to a gate
    pub fn lookup_gate(
        &mut self,
        a: Variable,
        b: Variable,
        c: Variable,
    ) -> Result<(), PreProcessingError> {
        self.w_l.push(a);
        self.w_l.push(b);
        self.w_l.push(c);
        self.w_4.push(self.zero_var);
        self.q_l.push(BlsScalar::zero());
        self.q_r.push(BlsScalar::zero());

        // Add selector vectors
        self.q_m.push(BlsScalar::zero());
        self.q_o.push(BlsScalar::zero());
        self.q_c.push(BlsScalar::zero());
        self.q_4.push(BlsScalar::zero());
        self.q_arith.push(BlsScalar::zero());

        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());
        self.q_lookup.push(BlsScalar::one());

        Ok(())
    }

    /// Adds a width-3 poly gate.
    /// This gate gives total freedom to the end user to implement the corresponding
    /// circuits in the most optimized way possible because the under has access to the
    /// whole set of variables, as well as selector coefficients that take part in the
    /// computation of the gate equation.
    ///
    /// The final constraint added will force the following:
    /// `(a * b) * q_m + a * q_l + b * q_r + q_c + PI + q_o * c = 0`.
    pub fn poly_gate(
        &mut self,
        a: Variable,
        b: Variable,
        c: Variable,
        q_m: BlsScalar,
        q_l: BlsScalar,
        q_r: BlsScalar,
        q_o: BlsScalar,
        q_c: BlsScalar,
        pi: BlsScalar,
    ) -> (Variable, Variable, Variable) {
        self.w_l.push(a);
        self.w_r.push(b);
        self.w_o.push(c);
        self.w_4.push(self.zero_var);
        self.q_l.push(q_l);
        self.q_r.push(q_r);

        // Add selector vectors
        self.q_m.push(q_m);
        self.q_o.push(q_o);
        self.q_c.push(q_c);
        self.q_4.push(BlsScalar::zero());
        self.q_arith.push(BlsScalar::one());

        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());
        self.q_lookup.push(BlsScalar::zero());

        self.public_inputs.push(pi);

        self.perm
            .add_variables_to_map(a, b, c, self.zero_var, self.n);
        self.n += 1;

        (a, b, c)
    }

    /// Adds a gate which is designed to constrain a `Variable` to have
    /// a specific constant value which is sent as a `BlsScalar`.
    pub fn constrain_to_constant(&mut self, a: Variable, constant: BlsScalar, pi: BlsScalar) {
        self.poly_gate(
            a,
            a,
            a,
            BlsScalar::zero(),
            BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            -constant,
            pi,
        );
    }

    /// Asserts that two variables are the same
    // XXX: Instead of wasting a gate, we can use the permutation polynomial to do this
    pub fn assert_equal(&mut self, a: Variable, b: Variable) {
        self.poly_gate(
            a,
            b,
            self.zero_var,
            BlsScalar::zero(),
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            BlsScalar::zero(),
        );
    }

    /// Adds a single dummy constraint
    pub fn add_one_dummy_constraint(&mut self) {
        self.q_m.push(BlsScalar::from(1));
        self.q_l.push(BlsScalar::from(2));
        self.q_r.push(BlsScalar::from(3));
        self.q_o.push(BlsScalar::from(4));
        self.q_c.push(BlsScalar::from(4));
        self.q_4.push(BlsScalar::one());
        self.q_arith.push(BlsScalar::one());
        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());
        self.q_lookup.push(BlsScalar::one());
        self.public_inputs.push(BlsScalar::zero());
        let var_six = self.add_input(BlsScalar::from(6));
        let var_one = self.add_input(BlsScalar::from(1));
        let var_seven = self.add_input(BlsScalar::from(7));
        let var_min_twenty = self.add_input(-BlsScalar::from(20));
        self.w_l.push(var_six);
        self.w_r.push(var_seven);
        self.w_o.push(var_min_twenty);
        self.w_4.push(var_one);
        self.perm
            .add_variables_to_map(var_six, var_seven, var_min_twenty, var_one, self.n);
        self.n += 1;
    }

    /// This function is used to add a blinding factor to the witness polynomials
    /// XXX: Split this into two separate functions and document
    /// XXX: We could add another section to add random witness variables, with selector polynomials all zero
    pub fn add_dummy_constraints(&mut self) {
        // Add a dummy constraint so that we do not have zero polynomials
        self.q_m.push(BlsScalar::from(1));
        self.q_l.push(BlsScalar::from(2));
        self.q_r.push(BlsScalar::from(3));
        self.q_o.push(BlsScalar::from(4));
        self.q_c.push(BlsScalar::from(4));
        self.q_4.push(BlsScalar::one());
        self.q_arith.push(BlsScalar::one());
        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());
        self.q_lookup.push(BlsScalar::one());
        self.public_inputs.push(BlsScalar::zero());
        let var_six = self.add_input(BlsScalar::from(6));
        let var_one = self.add_input(BlsScalar::from(1));
        let var_seven = self.add_input(BlsScalar::from(7));
        let var_min_twenty = self.add_input(-BlsScalar::from(20));
        self.w_l.push(var_six);
        self.w_r.push(var_seven);
        self.w_o.push(var_min_twenty);
        self.w_4.push(var_one);
        self.perm
            .add_variables_to_map(var_six, var_seven, var_min_twenty, var_one, self.n);
        self.n += 1;
        //Add another dummy constraint so that we do not get the identity permutation
        self.q_m.push(BlsScalar::from(1));
        self.q_l.push(BlsScalar::from(1));
        self.q_r.push(BlsScalar::from(1));
        self.q_o.push(BlsScalar::from(1));
        self.q_c.push(BlsScalar::from(127));
        self.q_4.push(BlsScalar::zero());
        self.q_arith.push(BlsScalar::one());
        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());
        self.q_lookup.push(BlsScalar::one());
        self.public_inputs.push(BlsScalar::zero());
        self.w_l.push(var_min_twenty);
        self.w_r.push(var_six);
        self.w_o.push(var_seven);
        self.w_4.push(self.zero_var);
        self.perm
            .add_variables_to_map(var_min_twenty, var_six, var_seven, self.zero_var, self.n);
        self.n += 1;
    }

    /// Adds a plookup gate to the circuit with its corresponding
    /// constraints.
    ///
    /// This type of gate is usually used when we need to have
    /// the largest amount of performance and the minimum circuit-size
    /// possible. Since it allows the end-user to set every selector coefficient
    /// as scaling value on the gate eq.
    pub fn plookup_gate(
        &mut self,
        a: Variable,
        b: Variable,
        c: Variable,
        d: Option<Variable>,
        pi: BlsScalar,
    ) -> Variable {
        // Check if advice wire has a value
        let d = match d {
            Some(var) => var,
            None => self.zero_var,
        };

        self.w_l.push(a);
        self.w_r.push(b);
        self.w_o.push(c);
        self.w_4.push(d);

        // Add selector vectors
        self.q_l.push(BlsScalar::zero());
        self.q_r.push(BlsScalar::zero());
        self.q_o.push(BlsScalar::zero());
        self.q_c.push(BlsScalar::zero());
        self.q_4.push(BlsScalar::zero());
        self.q_arith.push(BlsScalar::zero());
        self.q_m.push(BlsScalar::zero());
        self.q_range.push(BlsScalar::zero());
        self.q_logic.push(BlsScalar::zero());
        self.q_fixed_group_add.push(BlsScalar::zero());
        self.q_variable_group_add.push(BlsScalar::zero());

        // For a lookup gate, only one selector poly is
        // turned on as the output is inputted directly
        self.q_lookup.push(BlsScalar::one());

        self.public_inputs.push(pi);

        self.perm.add_variables_to_map(a, b, c, d, self.n);

        self.n += 1;

        c
    }
}

#[cfg(test)]
mod tests {
    use super::super::helper::*;
    use super::*;
    use crate::commitment_scheme::kzg10::PublicParameters;
    use crate::plookup::{PlookupTable4Arity, PreprocessedTable4Arity};
    use crate::proof_system::{PlookupProver, PlookupVerifier, Prover, Verifier};

    #[test]
    #[ignore]
    fn test_plookup_full() {
        let public_parameters = PublicParameters::setup(2 * 30, &mut rand::thread_rng()).unwrap();
        let mut composer = PlookupComposer::new();

        let mut lookup_table = PlookupTable4Arity::new();
        lookup_table.insert_multi_mul(0, 3);

        // Create a prover struct
        let mut prover = PlookupProver::new(b"test");

        // add tp trans
        prover.key_transcript(b"key", b"additional seed information");

        let output = lookup_table.lookup(BlsScalar::from(2), BlsScalar::from(3), BlsScalar::one());

        let two = composer.add_witness_to_circuit_description(BlsScalar::from(2));
        let three = composer.add_witness_to_circuit_description(BlsScalar::from(3));
        let result = composer.add_witness_to_circuit_description(output.unwrap());
        let one = composer.add_witness_to_circuit_description(BlsScalar::one());

        composer.plookup_gate(two, three, result, Some(one), BlsScalar::one());
        composer.plookup_gate(two, three, result, Some(one), BlsScalar::one());
        composer.plookup_gate(two, three, result, Some(one), BlsScalar::one());
        composer.plookup_gate(two, three, result, Some(one), BlsScalar::one());
        composer.plookup_gate(two, three, result, Some(one), BlsScalar::one());

        composer.big_add(
            (BlsScalar::one(), two),
            (BlsScalar::one(), three),
            None,
            BlsScalar::zero(),
            BlsScalar::zero(),
        );

        // Commit Key
        let (ck, _) = public_parameters.trim(2 * 20).unwrap();

        let preprocessed_table = PreprocessedTable4Arity::preprocess(lookup_table, &ck, 3);

        // Commit Key
        let (ck, _) = public_parameters.trim(2 * 20).unwrap();

        // Preprocess circuit
        prover.preprocess(&ck);

        // Once the prove method is called, the public inputs are cleared
        // So pre-fetch these before calling Prove
        let public_inputs = prover.cs.public_inputs.clone();

        (prover.prove(&ck), public_inputs);
    }

    #[test]
    /// Tests that a circuit initially has 3 gates
    fn test_initial_circuit_size() {
        let composer: PlookupComposer = PlookupComposer::new();
        // Circuit size is n+3 because
        // - We have an extra gate which forces the first witness to be zero. This is used when the advice wire is not being used.
        // - We have two gates which ensure that the permutation polynomial is not the identity and
        // - Another gate which ensures that the selector polynomials are not all zeroes
        assert_eq!(3, composer.circuit_size())
    }

    #[test]
    #[ignore]
    // XXX: Move this to integration tests
    fn test_plookup_proof() {
        let public_parameters = PublicParameters::setup(2 * 30, &mut rand::thread_rng()).unwrap();

        // Create a prover struct
        let mut prover = PlookupProver::new(b"demo");

        // Add gadgets
        dummy_gadget_plookup(4, prover.mut_cs());

        // Commit Key
        let (ck, _) = public_parameters.trim(2 * 20).unwrap();

        // Preprocess circuit
        prover.preprocess(&ck).unwrap();

        let public_inputs = prover.cs.public_inputs.clone();

        let proof = prover.prove(&ck).unwrap();

        // Create the public table with dummy rows matching dummy gates
        let mut plookup_table = PlookupTable4Arity::new();
        plookup_table.add_dummy_rows();

        // Verifier
        //
        let mut verifier = PlookupVerifier::new(b"demo");

        // Add gadgets
        dummy_gadget_plookup(4, verifier.mut_cs());

        // Commit and Verifier Key
        let (ck, vk) = public_parameters.trim(2 * 20).unwrap();

        // Preprocess
        verifier.preprocess(&ck).unwrap();

        assert!(verifier
            .verify(&proof, &vk, &public_inputs, &plookup_table)
            .is_ok());
    }

    #[test]
    // XXX: Move this to integration tests
    fn test_cube_table() {
        let mut cube_table = PlookupTable4Arity::new();
        cube_table.add_dummy_rows();
        for i in 0..(32 - cube_table.0.len() as u64) {
            cube_table.0.push([
                BlsScalar::from(i),
                BlsScalar::from(i * i * i),
                BlsScalar::zero(),
                BlsScalar::zero(),
            ]);
        }

        let res = gadget_plookup_tester(
            |composer| {
                let zero = composer.add_input(BlsScalar::zero());

                let one = composer.add_input(BlsScalar::one());
                let one_cubed = composer.add_input(BlsScalar::one());

                let nine = composer.add_input(BlsScalar::from(9));
                let nine_cubed = composer.add_input(BlsScalar::from(9 * 9 * 9));

                let ten = composer.add_input(BlsScalar::from(10));
                let ten_cubed = composer.add_input(BlsScalar::from(10 * 10 * 10));

                let twelve = composer.add_input(BlsScalar::from(12));
                let twelve_cubed = composer.add_input(BlsScalar::from(12 * 12 * 12));

                // Add lookup query gates
                composer.plookup_gate(one, one_cubed, zero, Some(zero), BlsScalar::zero());
                composer.plookup_gate(nine, nine_cubed, zero, Some(zero), BlsScalar::zero());
                composer.plookup_gate(ten, ten_cubed, zero, Some(zero), BlsScalar::zero());
                composer.plookup_gate(twelve, twelve_cubed, zero, Some(zero), BlsScalar::zero());

                // Sanity check that the inputs are of an expected size.
                // The cube root of 1729 is just over 12, so all inputs
                // ought to be less than 4 bits.

                composer.range_gate(one, 4);
                composer.range_gate(twelve, 4);
                composer.range_gate(nine, 4);
                composer.range_gate(ten, 4);

                // Checks that 1^3 + 12^3 = 1729 (public input)
                composer.poly_gate(
                    one_cubed,
                    twelve_cubed,
                    zero,
                    BlsScalar::zero(),
                    BlsScalar::one(),
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                    -BlsScalar::from(1729),
                );

                // Checks that 9^3 + 10^3 = 1729 (public input)
                composer.poly_gate(
                    nine_cubed,
                    ten_cubed,
                    zero,
                    BlsScalar::zero(),
                    BlsScalar::one(),
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                    -BlsScalar::from(1729),
                );

                // Now we must show that the two ways of writing 1729
                // as a sum of two cubes are indeed different. We can
                // show that set {a, b} =/= {c, d} by showing that
                // a =/= c, a =/= d, b =/= c, and b =/= d. We can show
                // an individual inequality a =/= c by asking the prover
                // to provide an inverse z to a - c. Then a constraint
                // shows that z * (a - c) = 1. If a = c it will be
                // impossible for the prover to provide such an inverse.

                let a_minus_c_inverse =
                    composer.add_input((BlsScalar::from(1) - BlsScalar::from(9)).invert().unwrap());

                let a_minus_d_inverse = composer
                    .add_input((BlsScalar::from(1) - BlsScalar::from(10)).invert().unwrap());

                let b_minus_c_inverse = composer
                    .add_input((BlsScalar::from(12) - BlsScalar::from(9)).invert().unwrap());

                let b_minus_d_inverse = composer.add_input(
                    (BlsScalar::from(12) - BlsScalar::from(10))
                        .invert()
                        .unwrap(),
                );

                let a_minus_c = composer.add(
                    (BlsScalar::one(), one),
                    (-BlsScalar::one(), nine),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                );

                let a_minus_d = composer.add(
                    (BlsScalar::one(), one),
                    (-BlsScalar::one(), ten),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                );

                let b_minus_c = composer.add(
                    (BlsScalar::one(), twelve),
                    (-BlsScalar::one(), nine),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                );

                let b_minus_d = composer.add(
                    (BlsScalar::one(), twelve),
                    (-BlsScalar::one(), ten),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                );

                // Each multiplication gate constrains the output to be constant 1
                composer.mul_gate(
                    a_minus_c,
                    a_minus_c_inverse,
                    zero,
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    -BlsScalar::one(),
                    BlsScalar::zero(),
                );

                composer.mul_gate(
                    a_minus_d,
                    a_minus_d_inverse,
                    zero,
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    -BlsScalar::one(),
                    BlsScalar::zero(),
                );

                composer.mul_gate(
                    b_minus_c,
                    b_minus_c_inverse,
                    zero,
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    -BlsScalar::one(),
                    BlsScalar::zero(),
                );

                composer.mul_gate(
                    b_minus_d,
                    b_minus_d_inverse,
                    zero,
                    BlsScalar::one(),
                    BlsScalar::zero(),
                    -BlsScalar::one(),
                    BlsScalar::zero(),
                );
            },
            32,
            cube_table,
        );

        assert!(res.is_ok());
    }

    #[test]
    // XXX: Move this to integration tests
    fn test_plookup_all_gates() {
        use rand::rngs::ThreadRng;
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let mut random_table = PlookupTable4Arity::new();
        random_table.add_dummy_rows();

        let table_length = rng.gen_range(10, 100);
        for i in 0..table_length {
            random_table.0.push([
                BlsScalar::random(&mut rng),
                BlsScalar::random(&mut rng),
                BlsScalar::random(&mut rng),
                BlsScalar::random(&mut rng),
            ]);
        }

        fn all_gates_circuit(
            table: &PlookupTable4Arity,
            composer: &mut PlookupComposer,
            rng: &mut ThreadRng,
        ) {
            let random_row = table.0[rng.gen_range(0, table.0.len()) as usize];
            let (r1, r2, r3, r4) = (random_row[0], random_row[1], random_row[2], random_row[3]);
            let r1_var = composer.add_input(r1);
            let r2_var = composer.add_input(r2);
            let r3_var = composer.add_input(r3);
            let r4_var = composer.add_input(r4);

            // add a lookup query gate from the table
            composer.plookup_gate(r1_var, r2_var, r3_var, Some(r4_var), BlsScalar::zero());

            // add some arithmetic gates using the random values
            let r1_r2_sum = composer.add(
                (BlsScalar::one(), r1_var),
                (BlsScalar::one(), r2_var),
                BlsScalar::zero(),
                BlsScalar::zero(),
            );

            let r3_r4_prod = composer.mul(
                BlsScalar::one(),
                r3_var,
                r4_var,
                BlsScalar::zero(),
                BlsScalar::zero(),
            );

            // test logic gates
            let r1_xor_r2 = composer.xor_gate(r1_var, r2_var, 8);
            let r1_and_zero = composer.and_gate(r1_var, composer.zero_var, 8);

            // test a range gate
            composer.range_gate(r1_xor_r2, 8);

            // test a boolean gate
            composer.boolean_gate(r1_and_zero);
        }

        // Common View
        let public_parameters = PublicParameters::setup(128, &mut rand::thread_rng()).unwrap();

        // Provers View
        let (proof, public_inputs) = {
            // Create a prover struct
            let mut prover = PlookupProver::new(b"all_gates");

            // Additionally key the transcript
            prover.key_transcript(b"key", b"additional seed information");

            all_gates_circuit(&random_table, prover.mut_cs(), &mut rng);

            // Commit Key
            let (ck, _) = public_parameters
                .trim(2 * prover.cs.circuit_size().next_power_of_two())
                .unwrap();

            // This ought to be added to shared preprocessing
            if prover.mut_cs().w_l.len() < random_table.0.len() {
                for i in 0..(random_table.0.len() - prover.mut_cs().w_l.len()) {
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
                prover.prove_with_table(&ck, &random_table).unwrap(),
                public_inputs,
            )
        };

        // Verifiers view
        //
        // Create a Verifier object
        let mut verifier = PlookupVerifier::new(b"all_gates");

        // Additionally key the transcript
        verifier.key_transcript(b"key", b"additional seed information");

        // Add gadgets
        all_gates_circuit(&random_table, verifier.mut_cs(), &mut rng);

        // This ought to be added to shared preprocessing
        if verifier.mut_cs().w_l.len() < random_table.0.len() {
            for i in 0..(random_table.0.len() - verifier.mut_cs().w_l.len()) {
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
        let res = verifier.verify(&proof, &vk, &public_inputs, &random_table);

        assert!(res.is_ok());
    }
}

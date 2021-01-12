// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(clippy::too_many_arguments)]
use crate::fft::{EvaluationDomain, Evaluations, Polynomial};
use crate::permutation::constants::{K1, K2, K3};
use dusk_bls12_381::BlsScalar;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ProverKey {
    pub left_sigma: (Polynomial, Evaluations),
    pub right_sigma: (Polynomial, Evaluations),
    pub out_sigma: (Polynomial, Evaluations),
    pub fourth_sigma: (Polynomial, Evaluations),
    pub linear_evaluations: Evaluations, // Evaluations of f(x) = X [XXX: Remove this and benchmark if it makes a considerable difference -- These are just the domain elements]
    pub h_1: (Polynomial, Evaluations),
    pub h_2: (Polynomial, Evaluations),
    pub t: (Polynomial, Evaluations),
}

impl ProverKey {
    pub(crate) fn compute_quotient_i(
        &self,
        index: usize,
        w_l_i: &BlsScalar,
        w_r_i: &BlsScalar,
        w_o_i: &BlsScalar,
        w_4_i: &BlsScalar,
        f_i: &BlsScalar,
        t_i: &BlsScalar,
        t_i_next: &BlsScalar,
        h_1_i: &BlsScalar,
        h_2_i: &BlsScalar,
        h_1_i_next: &BlsScalar,
        h_2_i_next: &BlsScalar,
        z_i: &BlsScalar,
        z_i_next: &BlsScalar,
        p_i: &BlsScalar,
        p_i_next: &BlsScalar,
        alpha: &BlsScalar,
        l1_alpha_sq: &BlsScalar,
        l1_alpha_4: &BlsScalar,
        ln_alpha_6: &BlsScalar,
        ln_alpha_7: &BlsScalar,
        beta: &BlsScalar,
        gamma: &BlsScalar,
        delta: &BlsScalar,
        epsilon: &BlsScalar,
        omega_roots: &BlsScalar,
    ) -> BlsScalar {
        let a = self.compute_quotient_identity_range_check_i(
            index, w_l_i, w_r_i, w_o_i, w_4_i, z_i, alpha, beta, gamma,
        );
        let b = self.compute_quotient_copy_range_check_i(
            index, w_l_i, w_r_i, w_o_i, w_4_i, z_i_next, alpha, beta, gamma,
        );
        let c = self.compute_lookup_quotient_identity_range_check_i(
            index,
            f_i,
            t_i,
            t_i_next,
            p_i,
            alpha,
            delta,
            epsilon,
            omega_roots,
        );
        let d = self.compute_lookup_quotient_copy_range_check_i(
            index,
            h_1_i,
            h_2_i,
            h_1_i_next,
            h_2_i_next,
            p_i_next,
            alpha,
            delta,
            epsilon,
            omega_roots,
        );
        let e = self.compute_quotient_term_check_first_la_grange_polys(
            z_i,
            p_i,
            l1_alpha_sq,
            l1_alpha_4,
        );
        let f = self.compute_quotient_last_la_grange_polys(p_i, ln_alpha_7);
        let g = self.compute_overlap_check(h_1_i, h_2_i_next, ln_alpha_6);

        a + b + c + d + e + f + g
    }

    // (a(x) + beta * X + gamma) (b(X) + beta * k1 * X + gamma) (c(X) + beta * k2 * X + gamma)(d(X) + beta * k3 * X + gamma)z(X) * alpha
    fn compute_quotient_identity_range_check_i(
        &self,
        index: usize,
        w_l_i: &BlsScalar,
        w_r_i: &BlsScalar,
        w_o_i: &BlsScalar,
        w_4_i: &BlsScalar,
        z_i: &BlsScalar,
        alpha: &BlsScalar,
        beta: &BlsScalar,
        gamma: &BlsScalar,
    ) -> BlsScalar {
        let x = self.linear_evaluations[index];

        (w_l_i + (beta * x) + gamma)
            * (w_r_i + (beta * K1 * x) + gamma)
            * (w_o_i + (beta * K2 * x) + gamma)
            * (w_4_i + (beta * K3 * x) + gamma)
            * z_i
            * alpha
    }

    // (x - omega^n) * p(x) * (1 + delta) * (epsilon + f(x))(epsilon(1 + delta) + t(x) + delta * t(x_omega)) * alpha^5
    fn compute_lookup_quotient_identity_range_check_i(
        &self,
        index: usize,
        f_i: &BlsScalar,
        t_i: &BlsScalar,
        t_i_next: &BlsScalar,
        p_i: &BlsScalar,
        alpha: &BlsScalar,
        delta: &BlsScalar,
        epsilon: &BlsScalar,
        omega_roots: &BlsScalar,
    ) -> BlsScalar {
        let x = self.linear_evaluations[index];
        let alpha_5 = alpha * alpha * alpha * alpha * alpha;

        // Compute multi use fn, 1 + delta
        let one_plus_delta = BlsScalar::one() + delta;

        let a_1 = x - omega_roots;
        let a_2 = epsilon + f_i;
        let a_3 = (epsilon * one_plus_delta) + t_i + (delta * t_i_next);

        a_1 * p_i * one_plus_delta * a_2 * a_3 * alpha_5
    }

    // (a(x) + beta * Sigma1(X) + gamma) (b(X) + beta * Sigma2(X) + gamma) (c(X) + beta * Sigma3(X) + gamma)(d(X) + beta * Sigma4(X) + gamma) Z(X.omega) * alpha
    fn compute_quotient_copy_range_check_i(
        &self,
        index: usize,
        w_l_i: &BlsScalar,
        w_r_i: &BlsScalar,
        w_o_i: &BlsScalar,
        w_4_i: &BlsScalar,
        z_i_next: &BlsScalar,
        alpha: &BlsScalar,
        beta: &BlsScalar,
        gamma: &BlsScalar,
    ) -> BlsScalar {
        let left_sigma_eval = self.left_sigma.1[index];
        let right_sigma_eval = self.right_sigma.1[index];
        let out_sigma_eval = self.out_sigma.1[index];
        let fourth_sigma_eval = self.fourth_sigma.1[index];

        let product = (w_l_i + (beta * left_sigma_eval) + gamma)
            * (w_r_i + (beta * right_sigma_eval) + gamma)
            * (w_o_i + (beta * out_sigma_eval) + gamma)
            * (w_4_i + (beta * fourth_sigma_eval) + gamma)
            * z_i_next
            * alpha;

        -product
    }

    // (x - omega^n) * p(x_omega) * (epsilon(1 + delta) + h_1(x) + delta * h_1(x_omega)) * (epsilon(1 + delta) + h_2(x) + delta * h_2(x_omega)) * alpha^5
    fn compute_lookup_quotient_copy_range_check_i(
        &self,
        index: usize,
        h_1_i: &BlsScalar,
        h_2_i: &BlsScalar,
        h_1_i_next: &BlsScalar,
        h_2_i_next: &BlsScalar,
        p_i_next: &BlsScalar,
        alpha: &BlsScalar,
        delta: &BlsScalar,
        epsilon: &BlsScalar,
        omega_roots: &BlsScalar,
    ) -> BlsScalar {
        let alpha_5 = alpha * alpha * alpha * alpha * alpha;

        // Compute multi use fn's
        let one_plus_delta = BlsScalar::one() + delta;
        let epsilon_one_plus_delta = epsilon * one_plus_delta;

        let x = self.linear_evaluations[index];
        let a_1 = x - omega_roots;
        let a_2 = epsilon_one_plus_delta + h_1_i + (delta * h_1_i_next);
        let a_3 = epsilon_one_plus_delta + h_2_i + (delta * h_2_i_next);

        let product = p_i_next * a_1 * a_2 * a_3 * alpha_5;

        -product
    }

    // L_1(X)[Z(X) - 1] + L_1(X)[P(X) - 1]
    fn compute_quotient_term_check_first_la_grange_polys(
        &self,
        z_i: &BlsScalar,
        p_i: &BlsScalar,
        l1_alpha_sq: &BlsScalar,
        l1_alpha_4: &BlsScalar,
    ) -> BlsScalar {
        let a_1 = (z_i - BlsScalar::one()) * l1_alpha_sq;
        let a_2 = (p_i - BlsScalar::one()) * l1_alpha_4;

        a_1 + a_2
    }

    fn compute_quotient_last_la_grange_polys(
        &self,
        p_i: &BlsScalar,
        ln_alpha_7: &BlsScalar,
    ) -> BlsScalar {
        (p_i - BlsScalar::one()) * ln_alpha_7
    }

    fn compute_overlap_check(
        &self,
        h_1_i: &BlsScalar,
        h_2_i_next: &BlsScalar,
        ln_alpha_6: &BlsScalar,
    ) -> &BlsScalar {
        &(&(h_1_i - h_2_i_next) * ln_alpha_6)
    }

    pub(crate) fn compute_linearisation(
        &self,
        z_challenge: &BlsScalar,
        (alpha, beta, gamma): (&BlsScalar, &BlsScalar, &BlsScalar),
        (a_eval, b_eval, c_eval, d_eval): (&BlsScalar, &BlsScalar, &BlsScalar, &BlsScalar),
        (sigma_1_eval, sigma_2_eval, sigma_3_eval): (&BlsScalar, &BlsScalar, &BlsScalar),
        z_eval: &BlsScalar,
        z_poly: &Polynomial,
    ) -> Polynomial {
        let a = self.compute_lineariser_identity_range_check(
            (&a_eval, &b_eval, &c_eval, &d_eval),
            z_challenge,
            (alpha, beta, gamma),
            z_poly,
        );
        let b = self.compute_lineariser_copy_range_check(
            (&a_eval, &b_eval, &c_eval),
            z_eval,
            &sigma_1_eval,
            &sigma_2_eval,
            &sigma_3_eval,
            (alpha, beta, gamma),
            &self.fourth_sigma.0,
        );

        let domain = EvaluationDomain::new(z_poly.degree()).unwrap();
        let c = self.compute_lineariser_check_is_one(&domain, z_challenge, &alpha.square(), z_poly);
        &(&a + &b) + &c
    }
    // (a_eval + beta * z_challenge + gamma)(b_eval + beta * K1 * z_challenge + gamma)(c_eval + beta * K2 * z_challenge + gamma) * alpha z(X)
    fn compute_lineariser_identity_range_check(
        &self,
        (a_eval, b_eval, c_eval, d_eval): (&BlsScalar, &BlsScalar, &BlsScalar, &BlsScalar),
        z_challenge: &BlsScalar,
        (alpha, beta, gamma): (&BlsScalar, &BlsScalar, &BlsScalar),
        z_poly: &Polynomial,
    ) -> Polynomial {
        let beta_z = beta * z_challenge;

        // a_eval + beta * z_challenge + gamma
        let mut a_0 = a_eval + beta_z;
        a_0 += gamma;

        // b_eval + beta * K1 * z_challenge + gamma
        let beta_z_k1 = K1 * beta_z;
        let mut a_1 = b_eval + beta_z_k1;
        a_1 += gamma;

        // c_eval + beta * K2 * z_challenge + gamma
        let beta_z_k2 = K2 * beta_z;
        let mut a_2 = c_eval + beta_z_k2;
        a_2 += gamma;

        // d_eval + beta * K3 * z_challenge + gamma
        let beta_z_k3 = K3 * beta_z;
        let mut a_3 = d_eval + beta_z_k3;
        a_3 += gamma;

        let mut a = a_0 * a_1;
        a *= a_2;
        a *= a_3;
        a *= alpha; // (a_eval + beta * z_challenge + gamma)(b_eval + beta * K1 * z_challenge + gamma)(c_eval + beta * K2 * z_challenge + gamma)(d_eval + beta * K3 * z_challenge + gamma) * alpha
        z_poly * &a // (a_eval + beta * z_challenge + gamma)(b_eval + beta * K1 * z_challenge + gamma)(c_eval + beta * K2 * z_challenge + gamma) * alpha z(X)
    }
    // -(a_eval + beta * sigma_1 + gamma)(b_eval + beta * sigma_2 + gamma) (c_eval + beta * sigma_3 + gamma) * beta *z_eval * alpha^2 * Sigma_4(X)
    fn compute_lineariser_copy_range_check(
        &self,
        (a_eval, b_eval, c_eval): (&BlsScalar, &BlsScalar, &BlsScalar),
        z_eval: &BlsScalar,
        sigma_1_eval: &BlsScalar,
        sigma_2_eval: &BlsScalar,
        sigma_3_eval: &BlsScalar,
        (alpha, beta, gamma): (&BlsScalar, &BlsScalar, &BlsScalar),
        fourth_sigma_poly: &Polynomial,
    ) -> Polynomial {
        // a_eval + beta * sigma_1 + gamma
        let beta_sigma_1 = beta * sigma_1_eval;
        let mut a_0 = a_eval + beta_sigma_1;
        a_0 += gamma;

        // b_eval + beta * sigma_2 + gamma
        let beta_sigma_2 = beta * sigma_2_eval;
        let mut a_1 = b_eval + beta_sigma_2;
        a_1 += gamma;

        // c_eval + beta * sigma_3 + gamma
        let beta_sigma_3 = beta * sigma_3_eval;
        let mut a_2 = c_eval + beta_sigma_3;
        a_2 += gamma;

        let beta_z_eval = beta * z_eval;

        let mut a = a_0 * a_1 * a_2;
        a *= beta_z_eval;
        a *= alpha; // (a_eval + beta * sigma_1 + gamma)(b_eval + beta * sigma_2 + gamma)(c_eval + beta * sigma_3 + gamma) * beta * z_eval * alpha

        fourth_sigma_poly * &-a // -(a_eval + beta * sigma_1 + gamma)(b_eval + beta * sigma_2 + gamma) (c_eval + beta * sigma_3 + gamma) * beta * z_eval * alpha^2 * Sigma_4(X)
    }

    fn compute_lineariser_check_is_one(
        &self,
        domain: &EvaluationDomain,
        z_challenge: &BlsScalar,
        alpha_sq: &BlsScalar,
        z_coeffs: &Polynomial,
    ) -> Polynomial {
        // Evaluate l_1(z)
        let l_1_z = domain.evaluate_all_lagrange_coefficients(*z_challenge)[0];

        z_coeffs * &(l_1_z * alpha_sq)
    }
}

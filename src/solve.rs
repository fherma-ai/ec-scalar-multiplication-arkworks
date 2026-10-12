//! N independent scalar multiplications, through arkworks' own group.
use ark_bn254::{Fq, Fr, G1Affine, G1Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInt, PrimeField, Zero};

use crate::fherma::{Inputs, Outputs, Point, Tensor};

/// The `i`-th row of a limb tensor as an element of the field `F`.
///
/// `from_bigint` refuses a value at or above the modulus. The generator
/// reduces before it writes, so a refusal here means the case is not the one
/// this specification describes — better said than folded away.
fn fe<F: PrimeField<BigInt = BigInt<4>>>(data: &[u64], i: usize, limbs: usize) -> F {
    let mut words = [0u64; 4];
    words[..limbs].copy_from_slice(&data[i * limbs..(i + 1) * limbs]);
    F::from_bigint(BigInt::new(words)).expect("a field element below the modulus")
}

#[derive(Default)]
pub struct State {
    limbs: usize,
}

pub fn init(p: &Point) -> State {
    State { limbs: p.L as usize }
}

pub fn run(state: &mut State, inp: &Inputs) -> Outputs {
    let limbs = state.limbs;
    let n = inp.ks.data.len() / limbs;

    let mut rx = Vec::with_capacity(n * limbs);
    let mut ry = Vec::with_capacity(n * limbs);

    // Batched, because a single inversion per point would be most of the cost
    // of the answer rather than of the multiplication: the products are taken
    // in projective coordinates and brought down to affine together.
    let products: Vec<G1Projective> = (0..n)
        .map(|i| {
            // `new_unchecked` because the generator takes every point from a
            // square root of the curve equation, so they are on the curve by
            // construction and BN254 has cofactor one.
            let base = G1Affine::new_unchecked(
                fe::<Fq>(&inp.pxs.data, i, limbs),
                fe::<Fq>(&inp.pys.data, i, limbs),
            );
            base * fe::<Fr>(&inp.ks.data, i, limbs)
        })
        .collect();

    for point in G1Projective::normalize_batch(&products) {
        // A product reaches infinity only when the scalar is zero; the
        // specification writes it as (0, 0), which is not on this curve and so
        // cannot be mistaken for a point of it.
        let (x, y) = point.xy().unwrap_or((Fq::zero(), Fq::zero()));
        rx.extend_from_slice(&x.into_bigint().0[..limbs]);
        ry.extend_from_slice(&y.into_bigint().0[..limbs]);
    }

    let shape = vec![n as i64, limbs as i64];
    Outputs {
        rxs: Tensor { shape: shape.clone(), data: rx },
        rys: Tensor { shape, data: ry },
    }
}

pub fn free(state: State) {
    drop(state);
}

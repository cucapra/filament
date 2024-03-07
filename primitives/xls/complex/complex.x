import apfloat
import std

pub type APFloat = apfloat::APFloat;

pub fn ComplexMult(
    left_r: APFloat<EXP_SIZE, FRACTION_SIZE>,
    left_i: APFloat<EXP_SIZE, FRACTION_SIZE>,
    right_r: APFloat<EXP_SIZE, FRACTION_SIZE>,
    right_i: APFloat<EXP_SIZE, FRACTION_SIZE>
) -> APFloat<EXP_SIZE, FRACTION_SIZE>[2] {
    let ac = apfloat::mul(left_r, right_r);
    let bd = apfloat::mul(left_i, right_i);
    let re = apfloat::sub(ac, bd);

    let ad = apfloat::mul(left_r, right_i);
    let bc = apfloat::mul(left_i, right_r);
    let im = apfloat::add(ad, bc);

    [re, im]
}
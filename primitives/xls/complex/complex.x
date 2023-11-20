import apfloat
import std

pub type APFloat = apfloat::APFloat;

pub fn ComplexMultXLS(
    left_r: APFloat<ESIZE, FSIZE>,
    left_i: APFloat<ESIZE, FSIZE>,
    right_r: APFloat<ESIZE, FSIZE>,
    right_i: APFloat<ESIZE, FSIZE>
) -> (APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>) {
    let ac = apfloat::mul(left_r, right_r);
    let bd = apfloat::mul(left_i, right_i);
    let re = apfloat::sub(ac, bd);

    let ad = apfloat::mul(left_r, right_i);
    let bc = apfloat::mul(left_i, right_r);
    let im = apfloat::add(ad, bc);

    [re, im]
}

pub fn ComplexAddXLS(
    left_r: APFloat<ESIZE, FSIZE>,
    left_i: APFloat<ESIZE, FSIZE>,
    right_r: APFloat<ESIZE, FSIZE>,
    right_i: APFloat<ESIZE, FSIZE>
) -> (APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>) {
    let ac = apfloat::add(left_r, right_r);
    let bd = apfloat::add(left_i, right_i);
    (ac, bd)
}

pub fn ComplexSubXLS(
    left_r: APFloat<ESIZE, FSIZE>,
    left_i: APFloat<ESIZE, FSIZE>,
    right_r: APFloat<ESIZE, FSIZE>,
    right_i: APFloat<ESIZE, FSIZE>
) -> (APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>) {
    let ac = apfloat::sub(left_r, right_r);
    let bd = apfloat::sub(left_i, right_i);
    (ac, bd)
}

pub fn ComplexNegXLS(
    left_r: APFloat<ESIZE, FSIZE>,
    left_i: APFloat<ESIZE, FSIZE>
) -> (APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>) {
    let a = apfloat::negate(left_r);
    let b = apfloat::negate(left_i);
    (a, b)
}
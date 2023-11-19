import apfloat
import std

pub type APFloat = apfloat::APFloat;

// output has form (out0_r, out0_i, out1_r, out1_i)
pub fn Butterfly(
    in0_r: APFloat<ESIZE, FSIZE>,
    in0_i: APFloat<ESIZE, FSIZE>,
    in1_r: APFloat<ESIZE, FSIZE>,
    in1_i: APFloat<ESIZE, FSIZE>,
    twd_r: APFloat<ESIZE, FSIZE>,
    twd_i: APFloat<ESIZE, FSIZE>
) -> (APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>, APFloat<ESIZE, FSIZE>) {
    // w * in1
    let ac = apfloat::mul(in1_r, twd_r);
    let bd = apfloat::mul(in1_i, twd_i);
    let re = apfloat::sub(ac, bd);
    
    let ad = apfloat::mul(in1_r, twd_i);
    let bc = apfloat::mul(in1_i, twd_r);
    let im = apfloat::add(ad, bc);

    // in0 + (w * in1)
    let out0_r = apfloat::add(in0_r, re);
    let out0_i = apfloat::add(in0_i, im);

    // in0 - (w * in1)
    let out1_r = apfloat::sub(in0_r, re);
    let out1_i = apfloat::sub(in0_i, im);

    (out0_r, out0_i, out1_r, out1_i)
}
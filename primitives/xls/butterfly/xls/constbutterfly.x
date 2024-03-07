import apfloat
import std

pub type float32 = apfloat::APFloat<8, 23>;

// output has form (out0_r, out0_i, out1_r, out1_i)
pub fn ConstButterfly(
    in0_r: float32,
    in0_i: float32,
    in1_r: float32,
    in1_i: float32
) -> (float32, float32, float32, float32) {
    // get w from integer value
    // uses esize and fsize as a hacky temp solution
    let twd_r_int = u32:ESIZE;
    let twd_r = float32 {
        sign: twd_r_int[31:32],
        bexp: twd_r_int[23:31],
        fraction: twd_r_int[0:23],
    };

    let twd_i_int = u32:FSIZE;
    let twd_i = float32 {
        sign: twd_i_int[31:32],
        bexp: twd_i_int[23:31],
        fraction: twd_i_int[0:23],
    };

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
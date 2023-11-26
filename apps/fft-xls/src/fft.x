import float32
import std

pub type F32 = float32::F32;
pub type Complex = (F32, F32);

pub fn StridePermutation<NStages:u32, NPoints:u32>(
    inp: Complex[NPoints]
) -> Complex[NPoints] {
    for (i, out_array) in u32:0 .. NPoints/u32:2 {
        let (re_0, im_0) = inp[i*2];
        let (re_1, im_1) = inp[i*2+1];
        let out_0 = update(out_array, i, (re_0, im_0));
        update(out_0, i+NPoints/u32:2, (re_1, im_1))
    } (Complex[NPoints]: [(float32::zero(u1:0), float32::zero(u1:0)), ...])
}

pub fn BitRevHelper<NStages:u32>(
    n: u32,
) -> u32 {
    let (x, rev_ret) = for (i, (n_acc, rev)) : (u32, (u32, u32)) in u32:0 .. NStages {
        let new_rev_0 = rev << u32:1;
        let n_0 = n & u32:1;
        let new_rev = new_rev_0 | n_0;
        let new_n = n >> u32:1;
        (new_n, new_rev)
    } ((n, u32:0));
    rev_ret
}

pub fn BitRev<NStages:u32, NPoints:u32>(
    inp: (F32, F32)[NPoints]
) -> (F32, F32)[NPoints] {
    let array_init: Complex[NPoints] = Complex[NPoints]: [(float32::zero(u1:0), float32::zero(u1:0)), ...];

    for (i, out_array) in u32:0 .. NPoints {
        let br_i = BitRevHelper<NStages>(i);
        let (re, im) = inp[br_i];
        update(out_array, i, (re, im))
    } (array_init)
}

pub fn Butterfly(
    in0_r: F32,
    in0_i: F32,
    in1_r: F32,
    in1_i: F32,
    twd_r: F32,
    twd_i: F32
) -> ((F32, F32), (F32, F32)) { // (out0_r, out0_i, out1_r, out1_i)
    // w * in1
    let ac = float32::mul(in1_r, twd_r);
    let bd = float32::mul(in1_i, twd_i);
    let re = float32::sub(ac, bd);

    let ad = float32::mul(in1_r, twd_i);
    let bc = float32::mul(in1_i, twd_r);
    let im = float32::add(ad, bc);

    // in0 + (w * in1)
    let out0_r = float32::add(in0_r, re);
    let out0_i = float32::add(in0_i, im);

    // in0 - (w * in1)
    let out1_r = float32::sub(in0_r, re);
    let out1_i = float32::sub(in0_i, im);

    ((out0_r, out0_i), (out1_r, out1_i))
}

pub fn Butterflies<NPoints:u32, TPoints:u32>(
    inp: Complex[NPoints],
    twiddle: Complex[TPoints]
) -> Complex[NPoints] {
    let array_init: Complex[NPoints] = Complex[NPoints]: [(float32::zero(u1:0), float32::zero(u1:0)), ...];

    for (i, out_arr) in u32:0 .. NPoints {
        let (in0_r, in0_i) = inp[u32:2*i];
        let (in1_r, in1_i) = inp[u32:2*i+1];
        let (twd_r, twd_i) = twiddle[i];
        let (b0, b1) = Butterfly(in0_r, in0_i, in1_r, in1_i, twd_r, twd_i);
        let out_arr0 = update(out_arr, u32:2*i, b0);
        update(out_arr0, u32:2*i+u32:1, b1)
    } (array_init)

    //array_init
}

pub fn FFT<NStages:u32, NPoints: u32, TPoints:u32>(
    inp: Complex[NPoints], // real and imaginary 
    twiddle_in: Complex[NStages][TPoints], // real and imaginary
) -> Complex[NPoints] {    // real and imaginary
    // first, bit reverse input
    let bitrev_out = BitRev<NStages, NPoints>(inp);

    let stage_array_single_init: Complex[NPoints] = 
        Complex[NPoints]: [(float32::zero(u1:0), float32::zero(u1:0)), ...];

    let stage_array_init: Complex[NPoints][NStages] = 
        Complex[NPoints][NStages]: [stage_array_single_init, ...];

    let stage_array_in_init: Complex[NPoints][NStages+u32:1] = 
        Complex[NPoints][NStages+u32:1]: [stage_array_single_init, ...];

    // input to first round of butterflies is bitrev
    let stage_array_butterfly_init = update(stage_array_in_init, u32:0, bitrev_out);

    // do butterflies and stride permutations
    let (arr_in, arr_out) = for (r_stage, (stage_array_in, stage_array_out)) in u32:0 .. NStages {
        //let butterfly_out: Complex[NPoints] = Butterflies<NPoints, TPoints>(stage_array_in, twiddle_in[r_stage]);

        // this was originally a call to Butterflies() -> that was segfaulting for some reason 
        // works now that it is inlined lol
        let twiddle = twiddle_in[r_stage];
        let array_init: Complex[NPoints] = Complex[NPoints]: [(float32::zero(u1:0), float32::zero(u1:0)), ...];

        let butterfly_out: Complex[NPoints] = for (i, out_arr) in u32:0 .. NPoints {
            let (in0_r, in0_i) = stage_array_in[r_stage][u32:2*i];
            let (in1_r, in1_i) = stage_array_in[r_stage][u32:2*i+1];
            let (twd_r, twd_i) = twiddle_in[r_stage][i];
            let (b0, b1) = Butterfly(in0_r, in0_i, in1_r, in1_i, twd_r, twd_i);
            let out_arr0 = update(out_arr, u32:2*i, b0);
            update(out_arr0, u32:2*i+u32:1, b1)
        } (array_init);

        let out_updated = update(stage_array_out, r_stage, butterfly_out);
        let sp_out = StridePermutation<NStages, NPoints>(butterfly_out);
        let in_updated = update(stage_array_in, r_stage+u32:1, sp_out);
        (in_updated, out_updated)
    } ((stage_array_butterfly_init, stage_array_init));

    arr_in[NStages]
}

pub fn XLSFFT(
    inp: (F32, F32)[16],
    twiddle_in: (F32, F32)[4][8]
) -> (F32, F32)[16] {
    FFT<u32:4, u32:16, u32:8>(inp, twiddle_in)
}
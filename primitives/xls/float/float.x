import apfloat
import std

pub type APFloat = apfloat::APFloat;

pub fn FPMultXLS(
    left: APFloat<ESIZE, FSIZE>,
    right: APFloat<ESIZE, FSIZE>
) -> APFloat<ESIZE, FSIZE> {
    apfloat::mul(left, right)
}

pub fn FPAddXLS(
    left: APFloat<ESIZE, FSIZE>,
    right: APFloat<ESIZE, FSIZE>
) -> APFloat<ESIZE, FSIZE> {
    apfloat::add(left, right)
}

pub fn FPSubXLS(
    left: APFloat<ESIZE, FSIZE>,
    right: APFloat<ESIZE, FSIZE>
) -> APFloat<ESIZE, FSIZE> {
    apfloat::sub(left, right)
}

pub fn FPNegXLS(
    in: APFloat<ESIZE, FSIZE>
) -> APFloat<ESIZE, FSIZE> {
    apfloat::neg(left)
}